-- 0041 · Comunicação com o paciente por WhatsApp (docs/25 MVP: confirmação, laudo pronto, opt-out;
-- docs/21: mensagens enviadas são rastreadas). Mensagem nunca leva dado de saúde (nome do exame,
-- resultado): só horário, primeiro nome e aviso de que há algo no portal.

CREATE SCHEMA comunicacao;
GRANT USAGE ON SCHEMA comunicacao TO vidacor_app;

-- Preferência do paciente: sem registro, WhatsApp permitido para avisos operacionais (consentimento
-- colhido no cadastro, docs/04); "SAIR" ou a recepção desligam. Cada mudança sobe a versão e vai ao lastro.
CREATE TABLE comunicacao.preferencia (
  paciente_id uuid PRIMARY KEY REFERENCES identidade.paciente(id),
  whatsapp boolean NOT NULL,
  motivo text NOT NULL CHECK (length(trim(motivo)) >= 3),
  alterado_em timestamptz NOT NULL DEFAULT now(),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);

-- Cada mensagem (enviada ou não) fica registrada; uma por modelo e referência (G3: repetir não duplica).
CREATE TABLE comunicacao.mensagem (
  id uuid PRIMARY KEY,
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  canal text NOT NULL CHECK (canal IN ('whatsapp')),
  modelo text NOT NULL CHECK (modelo IN ('confirmacao','laudo_pronto','reserva_expirada')),
  referencia uuid NOT NULL,
  situacao text NOT NULL CHECK (situacao IN ('enviada','bloqueada_opt_out','sem_celular')),
  chave_idempotencia uuid NOT NULL UNIQUE,
  provedor_id text,
  respondida_em timestamptz,
  resposta text CHECK (resposta IS NULL OR resposta IN ('confirmou','saiu')),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  UNIQUE (modelo, referencia)
);
CREATE INDEX mensagem_paciente_idx ON comunicacao.mensagem (paciente_id, criado_em);

ALTER TABLE comunicacao.preferencia ENABLE ROW LEVEL SECURITY;
ALTER TABLE comunicacao.preferencia FORCE ROW LEVEL SECURITY;
ALTER TABLE comunicacao.mensagem ENABLE ROW LEVEL SECURITY;
ALTER TABLE comunicacao.mensagem FORCE ROW LEVEL SECURITY;
CREATE POLICY preferencia_unidade ON comunicacao.preferencia
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'));
CREATE POLICY mensagem_unidade ON comunicacao.mensagem
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema'));
GRANT SELECT, INSERT, UPDATE ON comunicacao.preferencia, comunicacao.mensagem TO vidacor_app;
