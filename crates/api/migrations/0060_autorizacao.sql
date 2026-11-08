-- 0060 · Autorização pedida pelo sistema (docs/22 E4, docs/25 § 3.3, DEC-2026-11-08-80).
-- Revisão humana obrigatória antes de rodar fora de dev (G6).
--
-- O pedido nasce do agendamento (ou da reserva provisória) e vai à operadora pelo webservice TISS
-- (outbox, chave por pedido) ou é feito no portal da operadora e registrado com o protocolo. Cada
-- resposta é gravada como recebida (bruta, com SHA-256) antes de ser interpretada; a guia é
-- preenchida a partir dela na mesma transação. Nada é apagado.

-- O contrato diz se a operadora recebe pedido de autorização pelo webservice TISS.
ALTER TABLE faturamento.contrato ADD COLUMN autorizacao_por_webservice boolean NOT NULL DEFAULT false;

CREATE TABLE convenio.autorizacao (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  agendamento_id uuid NOT NULL REFERENCES agenda.agendamento(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  -- Cópia N1 para os dois identificadores na lista (o faturista não lê o cadastro).
  paciente_nome text NOT NULL,
  paciente_nascimento date NOT NULL,
  operadora text NOT NULL CHECK (length(trim(operadora)) BETWEEN 2 AND 80),
  contrato_id uuid REFERENCES faturamento.contrato(id),
  canal text NOT NULL CHECK (canal IN ('webservice','portal')),
  carteirinha text NOT NULL CHECK (carteirinha ~ '^[A-Za-z0-9.-]{1,30}$'),
  solicitante_nome text NOT NULL CHECK (length(trim(solicitante_nome)) BETWEEN 3 AND 70),
  solicitante_conselho text NOT NULL CHECK (solicitante_conselho ~ '^[0-9]{2}$'),
  solicitante_numero text NOT NULL CHECK (solicitante_numero ~ '^[A-Za-z0-9./-]{1,15}$'),
  solicitante_uf text NOT NULL CHECK (solicitante_uf ~ '^[0-9]{2}$'),
  solicitante_cbos text NOT NULL CHECK (solicitante_cbos ~ '^[0-9]{6}$'),
  indicacao_clinica text CHECK (length(indicacao_clinica) <= 500),
  situacao text NOT NULL CHECK (situacao IN ('aguardando_envio','aguardando_portal','enviada','em_analise',
    'pendente_documento','autorizada','parcial','negada','cancelada','recusada')),
  -- Mensagem TISS enviada (webservice) e a chave de idempotência do envio, gerada uma vez (G3).
  solicitacao bytea CHECK (octet_length(solicitacao) <= 1048576),
  solicitacao_sha256 text CHECK (solicitacao_sha256 ~ '^[0-9a-f]{64}$'),
  chave_envio uuid NOT NULL UNIQUE,
  protocolo text CHECK (protocolo ~ '^[A-Za-z0-9./-]{1,40}$'),
  enviada_em timestamptz,
  motivo text CHECK (length(motivo) <= 1000),
  guia_id uuid UNIQUE REFERENCES convenio.guia(id),
  respondida_em timestamptz,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK ((canal = 'webservice') = (solicitacao IS NOT NULL AND solicitacao_sha256 IS NOT NULL)),
  CHECK (situacao NOT IN ('autorizada','parcial') OR guia_id IS NOT NULL)
);
-- Um pedido em andamento por agendamento (G2): dois pedidos simultâneos dão um só. Depois de negado,
-- cancelado ou recusado, pode haver pedido novo.
CREATE UNIQUE INDEX autorizacao_uma_ativa ON convenio.autorizacao (agendamento_id)
  WHERE situacao NOT IN ('negada','cancelada','recusada');
CREATE INDEX autorizacao_situacao_idx ON convenio.autorizacao (unidade_id, situacao);

CREATE TABLE convenio.autorizacao_procedimento (
  autorizacao_id uuid NOT NULL REFERENCES convenio.autorizacao(id),
  tuss text NOT NULL CHECK (tuss ~ '^[0-9]{8}$'),
  descricao text NOT NULL,
  solicitado int NOT NULL CHECK (solicitado BETWEEN 1 AND 99),
  autorizado int CHECK (autorizado BETWEEN 0 AND 99),
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  PRIMARY KEY (autorizacao_id, tuss)
);

-- Cada resposta como recebida: bruta e com SHA-256, gravada antes de interpretar. A mesma resposta
-- repetida (mesmo conteúdo) não entra de novo.
CREATE TABLE convenio.autorizacao_resposta (
  id uuid PRIMARY KEY,
  autorizacao_id uuid NOT NULL REFERENCES convenio.autorizacao(id),
  origem_resposta text NOT NULL CHECK (origem_resposta IN ('webservice','portal')),
  bruta bytea NOT NULL CHECK (octet_length(bruta) BETWEEN 1 AND 524288),
  sha256 text NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
  -- Leitura: situação lida (nula se ilegível) e erro de leitura.
  situacao_lida text CHECK (situacao_lida IN ('autorizada','parcial','negada','em_analise',
    'pendente_documento','cancelada','recusada')),
  erro_leitura text CHECK (length(erro_leitura) <= 500),
  hash_confere boolean,
  aplicada boolean NOT NULL,
  recebida_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  correlacao text NOT NULL,
  UNIQUE (autorizacao_id, sha256),
  CHECK ((situacao_lida IS NULL) = (erro_leitura IS NOT NULL))
);

CREATE FUNCTION convenio.somente_insercao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'resposta de autorização é somente inserção';
END$$;
CREATE FUNCTION convenio.proibir_apagar() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'pedido de autorização não é apagado';
END$$;
CREATE TRIGGER resposta_imutavel BEFORE UPDATE OR DELETE ON convenio.autorizacao_resposta
  FOR EACH ROW EXECUTE FUNCTION convenio.somente_insercao();
CREATE TRIGGER autorizacao_nao_apaga BEFORE DELETE ON convenio.autorizacao
  FOR EACH ROW EXECUTE FUNCTION convenio.proibir_apagar();
CREATE TRIGGER autorizacao_procedimento_nao_apaga BEFORE DELETE ON convenio.autorizacao_procedimento
  FOR EACH ROW EXECUTE FUNCTION convenio.proibir_apagar();

-- RLS: recepção e faturista pedem e registram; gerente lê; o núcleo envia e lê respostas.
DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY['autorizacao','autorizacao_procedimento','autorizacao_resposta'] LOOP
    EXECUTE format('ALTER TABLE convenio.%I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE convenio.%I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format($p$CREATE POLICY %I ON convenio.%I
      USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','faturista','gerente','sistema'))
      WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','faturista','sistema'))$p$,
      t || '_unidade', t);
  END LOOP;
END$$;
GRANT SELECT, INSERT, UPDATE ON convenio.autorizacao, convenio.autorizacao_procedimento TO vidacor_app;
GRANT SELECT, INSERT ON convenio.autorizacao_resposta TO vidacor_app;
