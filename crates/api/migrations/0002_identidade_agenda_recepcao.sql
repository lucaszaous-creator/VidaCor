-- 0002 · Identidade, paciente, agenda, recepção e exames do M0 (docs/29 § 6.3)
-- Toda tabela de negócio leva os metadados de lastro do docs/21 § 1.

CREATE TABLE identidade.unidade (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE, nome text NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL, versao int NOT NULL DEFAULT 1, correlacao text NOT NULL
);

CREATE TABLE identidade.usuario (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  login text NOT NULL UNIQUE, nome text NOT NULL,
  perfis text[] NOT NULL CHECK (cardinality(perfis) > 0),
  ativo boolean NOT NULL DEFAULT true,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
GRANT SELECT ON identidade.unidade, identidade.usuario TO vidacor_app;

-- Sessão: o navegador guarda só um segredo aleatório em cookie HttpOnly; o banco guarda o hash.
CREATE TABLE identidade.sessao (
  id uuid PRIMARY KEY,
  usuario_id uuid NOT NULL REFERENCES identidade.usuario(id),
  segredo_hash bytea NOT NULL UNIQUE,
  csrf text NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(),
  expira_em timestamptz NOT NULL,
  revogada_em timestamptz,
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL, versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL, criado_por uuid
);
CREATE INDEX sessao_usuario_idx ON identidade.sessao (usuario_id);
GRANT SELECT, INSERT, UPDATE ON identidade.sessao TO vidacor_app;

-- Pedido de login em andamento (state, nonce e verificador PKCE), de uso único e curto.
CREATE TABLE identidade.login_pendente (
  state text PRIMARY KEY,
  nonce text NOT NULL,
  verificador_pkce text NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(),
  expira_em timestamptz NOT NULL,
  usado_em timestamptz
);
GRANT SELECT, INSERT, UPDATE ON identidade.login_pendente TO vidacor_app;

CREATE TABLE identidade.paciente (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  nome text NOT NULL CHECK (length(nome) BETWEEN 3 AND 200),
  nascimento date NOT NULL CHECK (nascimento >= DATE '1900-01-01'),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);

CREATE TABLE agenda.sala (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE, nome text NOT NULL,
  ae_title text CHECK (ae_title ~ '^[A-Z0-9_]{1,16}$'),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);

CREATE TABLE agenda.agendamento (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  sala_id uuid NOT NULL REFERENCES agenda.sala(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  inicio timestamptz NOT NULL, fim timestamptz NOT NULL CHECK (fim > inicio),
  exame_tuss text NOT NULL CHECK (exame_tuss ~ '^[0-9]{8}$'),
  exame_nome text NOT NULL,
  situacao text NOT NULL CHECK (situacao IN
    ('agendado','confirmado','em_atendimento','concluido','faltou','cancelado')),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
CREATE INDEX agendamento_dia_idx ON agenda.agendamento (unidade_id, inicio);
CREATE INDEX agendamento_paciente_idx ON agenda.agendamento (paciente_id);
CREATE INDEX agendamento_sala_idx ON agenda.agendamento (sala_id);
-- Horário de sala: dois agendamentos ativos não começam no mesmo horário da mesma sala (G2).
CREATE UNIQUE INDEX agendamento_horario_sala_unico ON agenda.agendamento (sala_id, inicio)
  WHERE situacao NOT IN ('cancelado','faltou');

CREATE TABLE recepcao.atendimento (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  accession text NOT NULL UNIQUE,
  agendamento_id uuid NOT NULL REFERENCES agenda.agendamento(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  sala_id uuid NOT NULL REFERENCES agenda.sala(id),
  situacao text NOT NULL CHECK (situacao IN ('aguardando_exame','em_exame','exame_concluido','cancelado')),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
-- Um agendamento, um check-in (REQ-REC-002, G2: constraint única).
CREATE UNIQUE INDEX atendimento_um_por_agendamento ON recepcao.atendimento (agendamento_id);
CREATE INDEX atendimento_paciente_idx ON recepcao.atendimento (paciente_id);
CREATE INDEX atendimento_sala_idx ON recepcao.atendimento (sala_id);

CREATE TABLE exames.estudo (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  study_uid text NOT NULL UNIQUE CHECK (study_uid ~ '^[0-9.]{1,64}$'),
  orthanc_id text NOT NULL,
  accession text,
  atendimento_id uuid REFERENCES recepcao.atendimento(id),
  imagens int NOT NULL DEFAULT 0 CHECK (imagens >= 0),
  conciliado boolean NOT NULL DEFAULT false,
  recebido_em timestamptz NOT NULL DEFAULT now(),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  -- Estudo sem atendimento nunca está conciliado (imagem sem paciente não vai para laudo).
  CHECK (NOT conciliado OR atendimento_id IS NOT NULL)
);
CREATE INDEX estudo_atendimento_idx ON exames.estudo (atendimento_id);

GRANT SELECT, INSERT, UPDATE ON identidade.paciente, agenda.sala, agenda.agendamento,
  recepcao.atendimento, exames.estudo TO vidacor_app;
