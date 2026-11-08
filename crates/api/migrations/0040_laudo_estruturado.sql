-- 0040 · Laudo estruturado com classificações (docs/22 E2, docs/25 § 3.3, DEC-2026-11-08-64).
-- Cada achado pertence a uma versão do laudo e guarda os descritores marcados pelo médico, a
-- categoria e a conduta calculadas pelo núcleo e a versão da regra aplicada (ex.: TIRADS-ACR-2017.1).
-- Mudança de versão da regra não altera achados antigos. Achado de versão assinada é imutável;
-- nada é apagado: no rascunho o achado é "removido" com quem e quando.

CREATE TABLE laudos.achado (
  id uuid PRIMARY KEY,
  codigo text NOT NULL UNIQUE,
  laudo_id uuid NOT NULL REFERENCES laudos.laudo(id),
  versao_numero int NOT NULL,
  numero int NOT NULL CHECK (numero BETWEEN 1 AND 999),
  sistema text NOT NULL CHECK (sistema IN ('tirads','birads')),
  localizacao text NOT NULL CHECK (length(localizacao) BETWEEN 1 AND 120),
  -- Maior diâmetro em décimos de milímetro (sem ponto flutuante); BI-RADS sem massa não tem.
  maior_diametro_dmm int CHECK (maior_diametro_dmm BETWEEN 1 AND 1500),
  descritores jsonb NOT NULL CHECK (jsonb_typeof(descritores) = 'object'),
  pontos int CHECK (pontos BETWEEN 0 AND 20),
  categoria text NOT NULL CHECK (length(categoria) BETWEEN 1 AND 8),
  categoria_nome text NOT NULL,
  conduta text NOT NULL,
  regra_versao text NOT NULL CHECK (regra_versao ~ '^[A-Z]+-[A-Z0-9-]+\.[0-9]+$'),
  texto_achado text NOT NULL CHECK (length(texto_achado) <= 2000),
  texto_conclusao text NOT NULL CHECK (length(texto_conclusao) <= 1000),
  situacao text NOT NULL DEFAULT 'ativo' CHECK (situacao IN ('ativo','removido')),
  removido_em timestamptz, removido_por uuid REFERENCES identidade.usuario(id),
  -- Achado copiado da versão anterior na retificação: de onde veio.
  copiado_de uuid REFERENCES laudos.achado(id),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  FOREIGN KEY (laudo_id, versao_numero) REFERENCES laudos.versao (laudo_id, numero),
  UNIQUE (laudo_id, versao_numero, numero),
  CHECK ((sistema = 'tirads') = (pontos IS NOT NULL)),
  CHECK (sistema = 'birads' OR maior_diametro_dmm IS NOT NULL),
  CHECK ((situacao = 'removido') = (removido_em IS NOT NULL AND removido_por IS NOT NULL))
);
CREATE INDEX achado_laudo_idx ON laudos.achado (laudo_id, versao_numero) WHERE situacao = 'ativo';

-- Achado só entra ou muda em versão em rascunho; o único UPDATE aceito é "ativo" -> "removido";
-- DELETE nunca (CLAUDE.md: registro clínico não é apagado). Vale também para o dono do banco.
CREATE FUNCTION laudos.proteger_achado() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
  v_situacao text;
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'achado de laudo não é apagado';
  END IF;
  SELECT situacao INTO v_situacao FROM laudos.versao
   WHERE laudo_id = NEW.laudo_id AND numero = NEW.versao_numero;
  IF v_situacao IS DISTINCT FROM 'rascunho' THEN
    RAISE EXCEPTION 'achado só muda em versão de laudo em rascunho';
  END IF;
  IF TG_OP = 'UPDATE' THEN
    IF NOT (OLD.situacao = 'ativo' AND NEW.situacao = 'removido')
       OR (to_jsonb(NEW) - ARRAY['situacao','removido_em','removido_por','versao'])
          IS DISTINCT FROM (to_jsonb(OLD) - ARRAY['situacao','removido_em','removido_por','versao']) THEN
      RAISE EXCEPTION 'achado de laudo só pode ser removido no rascunho';
    END IF;
  END IF;
  RETURN NEW;
END$$;
CREATE TRIGGER achado_protegido BEFORE INSERT OR UPDATE OR DELETE ON laudos.achado
  FOR EACH ROW EXECUTE FUNCTION laudos.proteger_achado();

-- Mesmo nível do laudo (N4): lê quem lê o laudo (vínculo de cuidado ou acesso de emergência, pelo
-- RLS de laudos.laudo); grava só o médico atribuído ao laudo (ou o núcleo, na retificação).
ALTER TABLE laudos.achado ENABLE ROW LEVEL SECURITY;
ALTER TABLE laudos.achado FORCE ROW LEVEL SECURITY;
CREATE POLICY achado_leitura ON laudos.achado FOR SELECT
  USING (EXISTS (SELECT 1 FROM laudos.laudo l WHERE l.id = laudo_id));
CREATE POLICY achado_escrita ON laudos.achado FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND EXISTS (
    SELECT 1 FROM laudos.laudo l WHERE l.id = laudo_id AND (comum.ctx_tem_perfil('sistema')
      OR (comum.ctx_tem_perfil('medico') AND l.medico_atribuido_id = NULLIF(current_setting('app.usuario_id', true), '')::uuid))));
CREATE POLICY achado_atualizacao ON laudos.achado FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND EXISTS (
    SELECT 1 FROM laudos.laudo l WHERE l.id = laudo_id AND (comum.ctx_tem_perfil('sistema')
      OR (comum.ctx_tem_perfil('medico') AND l.medico_atribuido_id = NULLIF(current_setting('app.usuario_id', true), '')::uuid))))
  WITH CHECK (unidade_id = comum.ctx_unidade());
GRANT SELECT, INSERT, UPDATE ON laudos.achado TO vidacor_app;
