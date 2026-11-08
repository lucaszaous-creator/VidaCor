-- 0056 · Modelos de laudo e frases rápidas (Modelos-Laudo.dc.html; docs/04 Laudar; DEC-2026-11-08-100).
-- Modelo da clínica: médico propõe, a coordenação médica aprova; quem propõe não aprova (separação
-- de funções conferida no banco). Modelo pessoal: só o próprio médico, publica direto. Editar é
-- nova versão com motivo; nada é apagado; desativar exige motivo. Modelo e frase não têm dado de
-- paciente (texto padrão com campos variáveis entre colchetes). Cada uso de modelo num laudo fica
-- registrado com a versão do modelo usada.

CREATE TABLE laudos.modelo (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  tipo text NOT NULL CHECK (tipo IN ('clinica','pessoal')),
  -- Pessoal: o médico dono; da clínica: ninguém.
  dono_id uuid REFERENCES identidade.usuario(id),
  nome text NOT NULL CHECK (length(trim(nome)) BETWEEN 3 AND 120),
  exame text NOT NULL CHECK (length(trim(exame)) BETWEEN 3 AND 160),
  modalidade text NOT NULL CHECK (modalidade ~ '^[A-Z]{2,4}$'),
  derivado_de uuid REFERENCES laudos.modelo(id),
  ativo boolean NOT NULL DEFAULT true,
  desativado_em timestamptz, desativado_por uuid REFERENCES identidade.usuario(id),
  motivo_desativacao text CHECK (length(trim(motivo_desativacao)) BETWEEN 5 AND 500),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK ((tipo = 'pessoal') = (dono_id IS NOT NULL)),
  CHECK (ativo = (desativado_em IS NULL AND desativado_por IS NULL AND motivo_desativacao IS NULL))
);
-- Nome único entre os modelos ativos do mesmo dono (ou da clínica) na unidade.
CREATE UNIQUE INDEX modelo_nome_ativo ON laudos.modelo
  (unidade_id, COALESCE(dono_id, '00000000-0000-0000-0000-000000000000'::uuid), lower(nome)) WHERE ativo;

CREATE TABLE laudos.modelo_versao (
  id uuid PRIMARY KEY,
  modelo_id uuid NOT NULL REFERENCES laudos.modelo(id),
  numero int NOT NULL CHECK (numero BETWEEN 1 AND 9999),
  texto text NOT NULL CHECK (length(trim(texto)) BETWEEN 1 AND 20000),
  motivo text CHECK (length(trim(motivo)) BETWEEN 5 AND 500),
  situacao text NOT NULL CHECK (situacao IN ('proposta','publicada','substituida','recusada')),
  proposta_por uuid NOT NULL REFERENCES identidade.usuario(id),
  decidido_por uuid REFERENCES identidade.usuario(id),
  decidido_em timestamptz,
  motivo_decisao text CHECK (length(trim(motivo_decisao)) BETWEEN 5 AND 500),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  UNIQUE (modelo_id, numero),
  CHECK (numero = 1 OR motivo IS NOT NULL),
  -- Quem propõe não aprova nem recusa a própria proposta.
  CHECK (decidido_por IS NULL OR decidido_por <> proposta_por),
  CHECK (situacao <> 'recusada' OR (decidido_por IS NOT NULL AND decidido_em IS NOT NULL AND motivo_decisao IS NOT NULL))
);
-- No máximo uma versão publicada e uma proposta em aberto por modelo (G2).
CREATE UNIQUE INDEX modelo_versao_uma_publicada ON laudos.modelo_versao (modelo_id) WHERE situacao = 'publicada';
CREATE UNIQUE INDEX modelo_versao_uma_proposta ON laudos.modelo_versao (modelo_id) WHERE situacao = 'proposta';

-- Modelo não se apaga; os dados de identidade não mudam; a única mudança é desativar com motivo.
CREATE FUNCTION laudos.proteger_modelo() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'modelo de laudo não se apaga; desative com motivo' USING ERRCODE = '42501';
  END IF;
  IF (to_jsonb(NEW) - ARRAY['ativo','desativado_em','desativado_por','motivo_desativacao','versao'])
     IS DISTINCT FROM (to_jsonb(OLD) - ARRAY['ativo','desativado_em','desativado_por','motivo_desativacao','versao'])
     OR NOT (OLD.ativo AND NOT NEW.ativo) THEN
    RAISE EXCEPTION 'modelo de laudo só pode ser desativado com motivo' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER modelo_protegido BEFORE UPDATE OR DELETE ON laudos.modelo
  FOR EACH ROW EXECUTE FUNCTION laudos.proteger_modelo();

-- Versão de modelo: texto congelado; proposta -> publicada ou recusada; publicada -> substituída.
-- Na clínica toda versão nasce proposta e só é publicada por outra pessoa; no pessoal o dono
-- publica direto. Vale também para o dono do banco.
CREATE FUNCTION laudos.proteger_modelo_versao() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
  v_tipo text;
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'versão de modelo não se apaga' USING ERRCODE = '42501';
  END IF;
  SELECT m.tipo INTO v_tipo FROM laudos.modelo m WHERE m.id = NEW.modelo_id;
  IF TG_OP = 'INSERT' THEN
    IF NOT ((v_tipo = 'clinica' AND NEW.situacao = 'proposta')
            OR (v_tipo = 'pessoal' AND NEW.situacao = 'publicada' AND NEW.decidido_por IS NULL)) THEN
      RAISE EXCEPTION 'versão de modelo da clínica nasce proposta; a pessoal nasce publicada' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
  END IF;
  IF (to_jsonb(NEW) - ARRAY['situacao','decidido_por','decidido_em','motivo_decisao','versao'])
     IS DISTINCT FROM (to_jsonb(OLD) - ARRAY['situacao','decidido_por','decidido_em','motivo_decisao','versao']) THEN
    RAISE EXCEPTION 'versão de modelo é imutável' USING ERRCODE = '42501';
  END IF;
  IF OLD.situacao = 'proposta' AND NEW.situacao IN ('publicada','recusada') THEN
    IF NEW.decidido_por IS NULL OR NEW.decidido_em IS NULL OR NEW.decidido_por = NEW.proposta_por THEN
      RAISE EXCEPTION 'proposta de modelo é decidida por outra pessoa' USING ERRCODE = '23514';
    END IF;
  ELSIF OLD.situacao = 'publicada' AND NEW.situacao = 'substituida' THEN
    IF NEW.decidido_por IS DISTINCT FROM OLD.decidido_por OR NEW.decidido_em IS DISTINCT FROM OLD.decidido_em THEN
      RAISE EXCEPTION 'versão de modelo é imutável' USING ERRCODE = '42501';
    END IF;
  ELSE
    RAISE EXCEPTION 'transição de versão de modelo não permitida' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER modelo_versao_protegida BEFORE INSERT OR UPDATE OR DELETE ON laudos.modelo_versao
  FOR EACH ROW EXECUTE FUNCTION laudos.proteger_modelo_versao();

-- Frase rápida: atalho começa com ponto. Editar é nova frase que substitui a anterior (a anterior é
-- desativada com motivo); nada é apagado.
CREATE TABLE laudos.frase (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  tipo text NOT NULL CHECK (tipo IN ('clinica','pessoal')),
  dono_id uuid REFERENCES identidade.usuario(id),
  atalho text NOT NULL CHECK (atalho ~ '^\.[a-z0-9_-]{2,20}$'),
  texto text NOT NULL CHECK (length(trim(texto)) BETWEEN 3 AND 1000),
  substitui_id uuid UNIQUE REFERENCES laudos.frase(id),
  motivo text CHECK (length(trim(motivo)) BETWEEN 5 AND 500),
  ativo boolean NOT NULL DEFAULT true,
  desativado_em timestamptz, desativado_por uuid REFERENCES identidade.usuario(id),
  motivo_desativacao text CHECK (length(trim(motivo_desativacao)) BETWEEN 5 AND 600),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK ((tipo = 'pessoal') = (dono_id IS NOT NULL)),
  CHECK (substitui_id IS NULL OR motivo IS NOT NULL),
  CHECK (ativo = (desativado_em IS NULL AND desativado_por IS NULL AND motivo_desativacao IS NULL))
);
CREATE UNIQUE INDEX frase_atalho_ativo ON laudos.frase
  (unidade_id, COALESCE(dono_id, '00000000-0000-0000-0000-000000000000'::uuid), atalho) WHERE ativo;

CREATE FUNCTION laudos.proteger_frase() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'frase de laudo não se apaga; desative com motivo' USING ERRCODE = '42501';
  END IF;
  IF (to_jsonb(NEW) - ARRAY['ativo','desativado_em','desativado_por','motivo_desativacao','versao'])
     IS DISTINCT FROM (to_jsonb(OLD) - ARRAY['ativo','desativado_em','desativado_por','motivo_desativacao','versao'])
     OR NOT (OLD.ativo AND NOT NEW.ativo) THEN
    RAISE EXCEPTION 'frase de laudo só pode ser desativada com motivo' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER frase_protegida BEFORE UPDATE OR DELETE ON laudos.frase
  FOR EACH ROW EXECUTE FUNCTION laudos.proteger_frase();

-- Uso de modelo num laudo: qual versão do modelo entrou em qual versão do laudo, por quem. Só cresce.
CREATE TABLE laudos.uso_modelo (
  id uuid PRIMARY KEY,
  laudo_id uuid NOT NULL REFERENCES laudos.laudo(id),
  laudo_versao int NOT NULL,
  modelo_versao_id uuid NOT NULL REFERENCES laudos.modelo_versao(id),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  FOREIGN KEY (laudo_id, laudo_versao) REFERENCES laudos.versao (laudo_id, numero)
);
CREATE INDEX uso_modelo_laudo_idx ON laudos.uso_modelo (laudo_id);
CREATE FUNCTION laudos.uso_modelo_imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'uso de modelo é imutável' USING ERRCODE = '42501'; END $$;
CREATE TRIGGER uso_modelo_sem_alteracao BEFORE UPDATE OR DELETE ON laudos.uso_modelo
  FOR EACH ROW EXECUTE FUNCTION laudos.uso_modelo_imutavel();

-- RLS: modelos e frases não têm dado de paciente, mas são da equipe médica. Lê o médico (os da
-- clínica e os seus) e a coordenação (os da clínica). Escreve: o dono no pessoal; na clínica o médico
-- propõe e a coordenação decide e desativa.
CREATE FUNCTION laudos.ctx_usuario() RETURNS uuid LANGUAGE sql STABLE AS
$$ SELECT NULLIF(current_setting('app.usuario_id', true), '')::uuid $$;

ALTER TABLE laudos.modelo ENABLE ROW LEVEL SECURITY;
ALTER TABLE laudos.modelo FORCE ROW LEVEL SECURITY;
CREATE POLICY modelo_leitura ON laudos.modelo FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND (
    (tipo = 'clinica' AND comum.ctx_tem_perfil('medico','gerente'))
    OR (tipo = 'pessoal' AND dono_id = laudos.ctx_usuario() AND comum.ctx_tem_perfil('medico'))));
CREATE POLICY modelo_escrita ON laudos.modelo FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND criado_por = laudos.ctx_usuario() AND comum.ctx_tem_perfil('medico') AND (
    tipo = 'clinica' OR dono_id = laudos.ctx_usuario()));
CREATE POLICY modelo_atualizacao ON laudos.modelo FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND (
    (tipo = 'pessoal' AND dono_id = laudos.ctx_usuario() AND comum.ctx_tem_perfil('medico'))
    OR (tipo = 'clinica' AND comum.ctx_tem_perfil('gerente'))))
  WITH CHECK (unidade_id = comum.ctx_unidade());

ALTER TABLE laudos.modelo_versao ENABLE ROW LEVEL SECURITY;
ALTER TABLE laudos.modelo_versao FORCE ROW LEVEL SECURITY;
CREATE POLICY modelo_versao_leitura ON laudos.modelo_versao FOR SELECT
  USING (EXISTS (SELECT 1 FROM laudos.modelo m WHERE m.id = modelo_id));
CREATE POLICY modelo_versao_escrita ON laudos.modelo_versao FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND proposta_por = laudos.ctx_usuario() AND criado_por = laudos.ctx_usuario()
    AND comum.ctx_tem_perfil('medico')
    AND EXISTS (SELECT 1 FROM laudos.modelo m WHERE m.id = modelo_id AND m.ativo));
CREATE POLICY modelo_versao_atualizacao ON laudos.modelo_versao FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND EXISTS (
    SELECT 1 FROM laudos.modelo m WHERE m.id = modelo_id AND (
      (m.tipo = 'pessoal' AND m.dono_id = laudos.ctx_usuario())
      OR (m.tipo = 'clinica' AND comum.ctx_tem_perfil('gerente')))))
  WITH CHECK (unidade_id = comum.ctx_unidade());

ALTER TABLE laudos.frase ENABLE ROW LEVEL SECURITY;
ALTER TABLE laudos.frase FORCE ROW LEVEL SECURITY;
CREATE POLICY frase_leitura ON laudos.frase FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND (
    (tipo = 'clinica' AND comum.ctx_tem_perfil('medico','gerente'))
    OR (tipo = 'pessoal' AND dono_id = laudos.ctx_usuario() AND comum.ctx_tem_perfil('medico'))));
CREATE POLICY frase_escrita ON laudos.frase FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND criado_por = laudos.ctx_usuario() AND comum.ctx_tem_perfil('medico') AND (
    (tipo = 'pessoal' AND dono_id = laudos.ctx_usuario())
    OR (tipo = 'clinica' AND comum.ctx_tem_perfil('gerente'))));
CREATE POLICY frase_atualizacao ON laudos.frase FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND (
    (tipo = 'pessoal' AND dono_id = laudos.ctx_usuario() AND comum.ctx_tem_perfil('medico'))
    OR (tipo = 'clinica' AND comum.ctx_tem_perfil('gerente'))))
  WITH CHECK (unidade_id = comum.ctx_unidade());

-- Uso: grava o médico atribuído ao laudo; lê quem lê o laudo.
ALTER TABLE laudos.uso_modelo ENABLE ROW LEVEL SECURITY;
ALTER TABLE laudos.uso_modelo FORCE ROW LEVEL SECURITY;
CREATE POLICY uso_modelo_leitura ON laudos.uso_modelo FOR SELECT
  USING (EXISTS (SELECT 1 FROM laudos.laudo l WHERE l.id = laudo_id));
CREATE POLICY uso_modelo_escrita ON laudos.uso_modelo FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND criado_por = laudos.ctx_usuario() AND comum.ctx_tem_perfil('medico')
    AND EXISTS (SELECT 1 FROM laudos.laudo l WHERE l.id = laudo_id AND l.medico_atribuido_id = laudos.ctx_usuario()));

GRANT EXECUTE ON FUNCTION laudos.ctx_usuario() TO vidacor_app;
GRANT SELECT, INSERT, UPDATE ON laudos.modelo, laudos.modelo_versao, laudos.frase TO vidacor_app;
GRANT SELECT, INSERT ON laudos.uso_modelo TO vidacor_app;

-- Quantos laudos usaram cada modelo (só a contagem, acima do RLS do laudo; sem paciente).
CREATE FUNCTION laudos.usos_dos_modelos(p_ids uuid[])
RETURNS TABLE (modelo_id uuid, usos bigint)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, laudos, comum AS $$
  SELECT v.modelo_id, count(DISTINCT u.laudo_id)
    FROM laudos.uso_modelo u JOIN laudos.modelo_versao v ON v.id = u.modelo_versao_id
   WHERE v.modelo_id = ANY (p_ids) AND u.unidade_id = comum.ctx_unidade()
     AND comum.ctx_tem_perfil('medico','gerente')
   GROUP BY v.modelo_id
$$;
REVOKE ALL ON FUNCTION laudos.usos_dos_modelos(uuid[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION laudos.usos_dos_modelos(uuid[]) TO vidacor_app;
