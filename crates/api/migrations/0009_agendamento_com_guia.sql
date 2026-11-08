-- 0009 · Agendamento de convênio com guia (docs/21 § 3.3, docs/27 FC-AGD, docs/30 § 3).

ALTER TABLE agenda.agendamento DROP CONSTRAINT agendamento_situacao_check;
ALTER TABLE agenda.agendamento
  ADD CONSTRAINT agendamento_situacao_check CHECK (situacao IN
    ('reserva_provisoria','agendado','confirmado','em_atendimento','concluido','faltou','cancelado','expirada')),
  ADD COLUMN expira_em timestamptz,
  ADD COLUMN motivo text,
  ADD CONSTRAINT agendamento_reserva_tem_prazo CHECK (situacao <> 'reserva_provisoria' OR expira_em IS NOT NULL);
-- O horário só fica ocupado por agendamento ativo (reserva expirada devolve a vaga, FC-AGD-08).
DROP INDEX agenda.agendamento_horario_sala_unico;
CREATE UNIQUE INDEX agendamento_horario_sala_unico ON agenda.agendamento (sala_id, inicio)
  WHERE situacao NOT IN ('cancelado','faltou','expirada');
CREATE INDEX agendamento_reserva_vencendo_idx ON agenda.agendamento (expira_em) WHERE situacao = 'reserva_provisoria';

CREATE TABLE agenda.agendamento_procedimento (
  agendamento_id uuid NOT NULL REFERENCES agenda.agendamento(id),
  tuss text NOT NULL CHECK (tuss ~ '^[0-9]{8}$'),
  PRIMARY KEY (agendamento_id, tuss)
);
GRANT SELECT, INSERT ON agenda.agendamento_procedimento TO vidacor_app;

CREATE SCHEMA convenio;
GRANT USAGE ON SCHEMA convenio TO vidacor_app;
CREATE TABLE convenio.guia (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  operadora text NOT NULL CHECK (length(operadora) BETWEEN 2 AND 80),
  numero text NOT NULL CHECK (numero ~ '^[A-Za-z0-9-]{1,30}$'),
  carteirinha text NOT NULL CHECK (carteirinha ~ '^[A-Za-z0-9.-]{1,30}$'),
  senha text NOT NULL CHECK (senha ~ '^[A-Za-z0-9-]{1,30}$'),
  validade date NOT NULL,
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  solicitante text NOT NULL CHECK (length(solicitante) BETWEEN 3 AND 120),
  situacao text NOT NULL DEFAULT 'disponivel' CHECK (situacao IN ('disponivel','consumida')),
  -- A guia é consumida por um único agendamento (docs/21: não pode ser usada por dois atendimentos).
  agendamento_id uuid UNIQUE REFERENCES agenda.agendamento(id),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  UNIQUE (operadora, numero),
  CHECK ((situacao = 'consumida') = (agendamento_id IS NOT NULL))
);
CREATE TABLE convenio.guia_procedimento (
  guia_id uuid NOT NULL REFERENCES convenio.guia(id),
  tuss text NOT NULL CHECK (tuss ~ '^[0-9]{8}$'),
  autorizado int NOT NULL CHECK (autorizado BETWEEN 1 AND 99),
  saldo int NOT NULL CHECK (saldo >= 0),
  PRIMARY KEY (guia_id, tuss),
  CHECK (saldo <= autorizado)
);
ALTER TABLE agenda.agendamento ADD COLUMN guia_id uuid REFERENCES convenio.guia(id);

ALTER TABLE convenio.guia ENABLE ROW LEVEL SECURITY;
ALTER TABLE convenio.guia FORCE ROW LEVEL SECURITY;
CREATE POLICY guia_unidade ON convenio.guia
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','faturista','gerente','sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','faturista','gerente','sistema'));
GRANT SELECT, INSERT, UPDATE ON convenio.guia, convenio.guia_procedimento TO vidacor_app;
