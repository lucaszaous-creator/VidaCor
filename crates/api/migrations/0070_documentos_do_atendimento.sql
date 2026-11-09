-- 0070 · Documentos do atendimento (Preparo-Exame.dc.html e Documentos.dc.html; DEC-2026-11-09-106
-- a 108): orientação de preparo por procedimento, versionada, e três documentos novos registrados em
-- documentos.emissao (comprovante do agendamento com preparo, declaração de comparecimento e recibo).
-- Revisão humana obrigatória antes de rodar fora de dev (G6). Só acrescenta: uma tabela nova e a
-- troca da lista de tipos aceitos na emissão por uma lista maior (nenhuma linha existente deixa de
-- passar). Nada é apagado.

-- ---------------------------------------------------------------------------------------------
-- Preparo do exame: passos numerados que o paciente recebe no comprovante. Como o catálogo, é da
-- clínica (vale em todas as unidades). Versão nova com motivo substitui a vigente; a anterior fica
-- guardada e o comprovante já emitido continua apontando para a versão impressa.
CREATE TABLE agenda.preparo (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  tuss text NOT NULL REFERENCES agenda.procedimento(tuss),
  numero int NOT NULL CHECK (numero >= 1),
  passos text[] NOT NULL CHECK (cardinality(passos) BETWEEN 1 AND 12),
  chegar_antes_min int NOT NULL DEFAULT 15 CHECK (chegar_antes_min BETWEEN 0 AND 120),
  vigente boolean NOT NULL DEFAULT true,
  motivo text CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  -- Duas versões novas sobre a mesma anterior: só uma entra (G2).
  UNIQUE (tuss, numero),
  CHECK (numero = 1 OR motivo IS NOT NULL)
);
CREATE UNIQUE INDEX preparo_um_vigente ON agenda.preparo (tuss) WHERE vigente;

-- Cada passo tem de 3 a 300 caracteres (o CHECK de coluna não alcança os elementos do array).
CREATE FUNCTION agenda.preparo_conferir_passos() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF EXISTS (SELECT 1 FROM unnest(NEW.passos) p WHERE p IS NULL OR length(trim(p)) NOT BETWEEN 3 AND 300) THEN
    RAISE EXCEPTION 'cada passo do preparo tem de 3 a 300 caracteres' USING ERRCODE = '23514';
  END IF;
  RETURN NEW;
END$$;
CREATE TRIGGER preparo_passos BEFORE INSERT ON agenda.preparo
  FOR EACH ROW EXECUTE FUNCTION agenda.preparo_conferir_passos();
-- Versionado: a única mudança aceita é deixar de ser vigente; nada é apagado (função da 0062).
CREATE TRIGGER preparo_versionado BEFORE UPDATE OR DELETE ON agenda.preparo
  FOR EACH ROW EXECUTE FUNCTION documentos.proteger_versionado();

ALTER TABLE agenda.preparo ENABLE ROW LEVEL SECURITY;
ALTER TABLE agenda.preparo FORCE ROW LEVEL SECURITY;
-- Leitura: quem agenda ou recebe o paciente e a gerência (o texto vai para o paciente); o núcleo em
-- contexto de sistema também lê. Escrita: só a gerência, em nome da própria unidade.
CREATE POLICY preparo_leitura ON agenda.preparo FOR SELECT
  USING (comum.ctx_tem_perfil('recepcao','gerente','sistema'));
CREATE POLICY preparo_escrita ON agenda.preparo FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
CREATE POLICY preparo_atualizacao ON agenda.preparo FOR UPDATE
  USING (comum.ctx_tem_perfil('gerente'))
  WITH CHECK (comum.ctx_tem_perfil('gerente'));
GRANT SELECT, INSERT, UPDATE ON agenda.preparo TO vidacor_app;

-- ---------------------------------------------------------------------------------------------
-- Emissão: três tipos novos. O registro é o agendamento (comprovante), o atendimento (declaração) ou
-- o recebimento (recibo).
ALTER TABLE documentos.emissao DROP CONSTRAINT emissao_tipo_check;
ALTER TABLE documentos.emissao ADD CONSTRAINT emissao_tipo_check
  CHECK (tipo IN ('laudo_impresso','dossie_lastro','comprovante_agendamento','declaracao_comparecimento','recibo'));
