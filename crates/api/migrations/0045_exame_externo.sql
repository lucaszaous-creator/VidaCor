-- 0045 · Exame externo (docs/22 E3): importado de outra instituição, só leitura, NUNCA vira laudo da
-- VidaCor (fica em tabelas próprias: laudos.laudo só aponta para exames.estudo). A identificação da
-- origem fica guardada como chegou (IHE PIR: corrige-se para o cadastro, o original não se perde).

CREATE TABLE exames.externo (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  instituicao text NOT NULL CHECK (length(trim(instituicao)) BETWEEN 2 AND 200),
  origem text NOT NULL CHECK (origem IN ('link','arquivo','dicomweb')),
  nome_na_origem text NOT NULL CHECK (length(nome_na_origem) BETWEEN 1 AND 200),
  nascimento_na_origem date NOT NULL,
  id_na_origem text CHECK (id_na_origem IS NULL OR length(id_na_origem) <= 64),
  documento_conferido boolean NOT NULL CHECK (documento_conferido),
  conferido_por uuid NOT NULL REFERENCES identidade.usuario(id),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem_registro text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
CREATE TABLE exames.externo_estudo (
  id uuid PRIMARY KEY,
  externo_id uuid NOT NULL REFERENCES exames.externo(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  study_uid text NOT NULL CHECK (study_uid ~ '^[0-9.]{1,64}$'),
  descricao text NOT NULL CHECK (length(descricao) BETWEEN 1 AND 200),
  modalidade text NOT NULL CHECK (modalidade ~ '^[A-Z]{2,4}$'),
  data_estudo date NOT NULL,
  imagens int NOT NULL CHECK (imagens > 0 AND imagens <= 100000),
  sha256 text NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, versao int NOT NULL DEFAULT 1,
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), correlacao text NOT NULL,
  -- O mesmo estudo não entra duas vezes para o mesmo paciente (G2/G3).
  UNIQUE (paciente_id, study_uid)
);
-- Importado não se altera nem se apaga (só leitura).
CREATE FUNCTION exames.externo_imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'exame externo é somente leitura'; END $$;
CREATE TRIGGER externo_sem_alteracao BEFORE UPDATE OR DELETE ON exames.externo FOR EACH ROW EXECUTE FUNCTION exames.externo_imutavel();
CREATE TRIGGER externo_estudo_sem_alteracao BEFORE UPDATE OR DELETE ON exames.externo_estudo FOR EACH ROW EXECUTE FUNCTION exames.externo_imutavel();

ALTER TABLE exames.externo ENABLE ROW LEVEL SECURITY;
ALTER TABLE exames.externo FORCE ROW LEVEL SECURITY;
ALTER TABLE exames.externo_estudo ENABLE ROW LEVEL SECURITY;
ALTER TABLE exames.externo_estudo FORCE ROW LEVEL SECURITY;
-- Recepção importa; quem atende (recepção, técnico, médico, gerência) lê na unidade.
CREATE POLICY externo_unidade ON exames.externo
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente'));
CREATE POLICY externo_estudo_unidade ON exames.externo_estudo
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente'));
GRANT SELECT, INSERT ON exames.externo, exames.externo_estudo TO vidacor_app;
