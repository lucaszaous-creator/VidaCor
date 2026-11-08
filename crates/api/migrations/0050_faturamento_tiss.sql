-- 0050 · Faturamento TISS com auditoria antes do lote (docs/22 E5, docs/21 § 3.9, docs/25 § 3.3,
-- DEC-2026-11-08-65). Revisão humana obrigatória antes de rodar fora de dev (G6).
--
-- Conta (guia SP/SADT faturável) nasce do atendimento com guia consumida e exame feito; a auditoria
-- grava pendências com a versão da regra da operadora; só conta sem pendência que impede entra em
-- lote; o lote tem número atômico por contrato (G2) e o arquivo TISS imutável com hash; demonstrativo,
-- glosa e recurso ficam ligados à conta. Nada é apagado: correção gera versão com motivo.

CREATE SCHEMA faturamento;
GRANT USAGE ON SCHEMA faturamento TO vidacor_app;

-- Contrato da unidade com a operadora (Convenios-Tabelas). `operadora` é o mesmo texto da guia
-- (convenio.guia.operadora), comparado sem diferenciar maiúsculas.
CREATE TABLE faturamento.contrato (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  operadora text NOT NULL CHECK (length(trim(operadora)) BETWEEN 2 AND 80),
  registro_ans text NOT NULL CHECK (registro_ans ~ '^[0-9]{6}$'),
  codigo_prestador text NOT NULL CHECK (codigo_prestador ~ '^[A-Za-z0-9]{1,14}$'),
  nome_prestador text NOT NULL CHECK (length(trim(nome_prestador)) BETWEEN 1 AND 70),
  cnes text NOT NULL CHECK (cnes ~ '^[0-9]{7}$'),
  -- Conjunto de regras de auditoria (crate dominio-faturamento), ex.: AUD-UNIMED-2026.1.
  regra_versao text NOT NULL CHECK (regra_versao ~ '^AUD-[A-Z0-9]+-[0-9]{4}\.[0-9]+$'),
  prazo_envio_dias int NOT NULL CHECK (prazo_envio_dias BETWEEN 1 AND 365),
  prazo_recurso_dias int NOT NULL CHECK (prazo_recurso_dias BETWEEN 1 AND 365),
  -- Último número de lote usado: incremento atômico na mesma transação do lote (G2).
  ultimo_lote bigint NOT NULL DEFAULT 0 CHECK (ultimo_lote >= 0),
  ativo boolean NOT NULL DEFAULT true,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
CREATE UNIQUE INDEX contrato_operadora_unico ON faturamento.contrato (unidade_id, lower(trim(operadora)));

-- Tabela de preços negociada, versionada por contrato (nunca alterada: versão nova com vigência).
CREATE TABLE faturamento.tabela (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  contrato_id uuid NOT NULL REFERENCES faturamento.contrato(id),
  versao int NOT NULL CHECK (versao >= 1),
  vigente_desde date NOT NULL,
  descricao text NOT NULL CHECK (length(trim(descricao)) BETWEEN 3 AND 120),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  correlacao text NOT NULL,
  UNIQUE (contrato_id, versao)
);
CREATE TABLE faturamento.preco (
  tabela_id uuid NOT NULL REFERENCES faturamento.tabela(id),
  tuss text NOT NULL CHECK (tuss ~ '^[0-9]{8}$'),
  descricao text NOT NULL CHECK (length(trim(descricao)) BETWEEN 1 AND 150),
  valor_centavos bigint NOT NULL CHECK (valor_centavos > 0 AND valor_centavos < 10000000000),
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  PRIMARY KEY (tabela_id, tuss)
);

CREATE TABLE faturamento.lote (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  contrato_id uuid NOT NULL REFERENCES faturamento.contrato(id),
  numero bigint NOT NULL CHECK (numero >= 1),
  versao_tiss text NOT NULL CHECK (versao_tiss ~ '^[0-9]\.[0-9]{2}\.[0-9]{2}$'),
  quantidade_guias int NOT NULL CHECK (quantidade_guias BETWEEN 1 AND 100),
  valor_total_centavos bigint NOT NULL CHECK (valor_total_centavos >= 0),
  -- Arquivo TISS como foi gerado (ISO-8859-1), com o hash do epílogo e o SHA-256 do arquivo.
  arquivo bytea NOT NULL CHECK (octet_length(arquivo) BETWEEN 1 AND 10485760),
  hash_tiss text NOT NULL CHECK (hash_tiss ~ '^[0-9a-f]{32}$'),
  sha256 text NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
  situacao text NOT NULL DEFAULT 'gerado'
    CHECK (situacao IN ('gerado','aguardando_envio','enviado','demonstrativo_recebido')),
  -- Chave de idempotência do envio à operadora: gerada uma vez, reaproveitada nas retentativas (G3).
  chave_envio uuid NOT NULL UNIQUE,
  protocolo text CHECK (protocolo ~ '^[A-Za-z0-9-]{1,30}$'),
  enviado_em timestamptz,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  UNIQUE (contrato_id, numero),
  CHECK ((situacao IN ('enviado','demonstrativo_recebido')) = (protocolo IS NOT NULL AND enviado_em IS NOT NULL))
);

-- Conta: a guia SP/SADT faturável de um atendimento. Um atendimento, uma conta; uma guia autorizada,
-- uma conta (G2: constraint única). Nome e nascimento são cópia N1 para os dois identificadores na
-- tela do faturamento (o faturista não lê a agenda nem o atendimento, docs/18 § 4).
CREATE TABLE faturamento.conta (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  atendimento_id uuid NOT NULL UNIQUE REFERENCES recepcao.atendimento(id),
  guia_id uuid NOT NULL UNIQUE REFERENCES convenio.guia(id),
  contrato_id uuid REFERENCES faturamento.contrato(id),
  operadora text NOT NULL,
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  paciente_nome text NOT NULL,
  paciente_nascimento date NOT NULL,
  data_execucao date NOT NULL,
  numero_guia_operadora text NOT NULL,
  senha text NOT NULL,
  data_autorizacao date NOT NULL,
  validade_senha date NOT NULL,
  carteirinha text NOT NULL,
  solicitante_nome text NOT NULL CHECK (length(trim(solicitante_nome)) BETWEEN 1 AND 120),
  solicitante_conselho text CHECK (solicitante_conselho ~ '^[0-9]{2}$'),
  solicitante_numero text CHECK (solicitante_numero ~ '^[A-Za-z0-9./-]{1,15}$'),
  solicitante_uf text CHECK (solicitante_uf ~ '^[0-9]{2}$'),
  solicitante_cbos text CHECK (solicitante_cbos ~ '^[0-9]{6}$'),
  cid text CHECK (cid ~ '^[A-Z][0-9]{2,3}$'),
  assinatura_beneficiario boolean NOT NULL DEFAULT false,
  tabela_id uuid REFERENCES faturamento.tabela(id),
  valor_total_centavos bigint NOT NULL DEFAULT 0 CHECK (valor_total_centavos >= 0),
  valor_pago_centavos bigint CHECK (valor_pago_centavos >= 0),
  situacao text NOT NULL DEFAULT 'em_conferencia' CHECK (situacao IN
    ('em_conferencia','pronta','com_pendencia','em_lote','enviada','paga','paga_com_glosa','glosada')),
  regra_versao text, auditada_em timestamptz, auditada_por uuid,
  lote_id uuid REFERENCES faturamento.lote(id),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK ((situacao IN ('em_conferencia','pronta','com_pendencia')) = (lote_id IS NULL))
);
CREATE INDEX conta_situacao_idx ON faturamento.conta (unidade_id, situacao);
CREATE INDEX conta_contrato_pronta_idx ON faturamento.conta (contrato_id) WHERE situacao = 'pronta';

CREATE TABLE faturamento.conta_item (
  conta_id uuid NOT NULL REFERENCES faturamento.conta(id),
  sequencial int NOT NULL CHECK (sequencial BETWEEN 1 AND 99),
  tuss text NOT NULL CHECK (tuss ~ '^[0-9]{8}$'),
  descricao text NOT NULL CHECK (length(trim(descricao)) BETWEEN 1 AND 150),
  quantidade int NOT NULL CHECK (quantidade BETWEEN 1 AND 999),
  -- Nulo quando o código não está na tabela da operadora (pendência que bloqueia).
  valor_unitario_centavos bigint CHECK (valor_unitario_centavos > 0),
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  PRIMARY KEY (conta_id, sequencial)
);

-- Estado anterior de cada correção (append-only): quem, quando, por quê e o que era.
CREATE TABLE faturamento.conta_historico (
  id uuid PRIMARY KEY,
  conta_id uuid NOT NULL REFERENCES faturamento.conta(id),
  versao int NOT NULL CHECK (versao >= 1),
  motivo text NOT NULL CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  anterior jsonb NOT NULL CHECK (jsonb_typeof(anterior) = 'object'),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  correlacao text NOT NULL,
  UNIQUE (conta_id, versao)
);

-- Pendências de cada conferência, com a versão da regra que as produziu. A conferência seguinte
-- marca as anteriores como superadas (nada é apagado).
CREATE TABLE faturamento.pendencia (
  id uuid PRIMARY KEY,
  conta_id uuid NOT NULL REFERENCES faturamento.conta(id),
  regra text NOT NULL CHECK (regra ~ '^[a-z-]{3,60}$'),
  regra_versao text NOT NULL,
  efeito text NOT NULL CHECK (efeito IN ('bloqueia','corrigir_antes','atencao')),
  origem_regra text NOT NULL,
  titulo text NOT NULL,
  detalhe text NOT NULL CHECK (length(detalhe) <= 500),
  o_que_fazer text NOT NULL,
  situacao text NOT NULL DEFAULT 'aberta' CHECK (situacao IN ('aberta','superada')),
  superada_em timestamptz,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  correlacao text NOT NULL,
  CHECK ((situacao = 'superada') = (superada_em IS NOT NULL))
);
CREATE INDEX pendencia_aberta_idx ON faturamento.pendencia (conta_id) WHERE situacao = 'aberta';

-- Uma conta entra em um lote só (G2: constraint única); com a guia única na conta, a mesma guia
-- não vai em dois lotes.
CREATE TABLE faturamento.lote_item (
  lote_id uuid NOT NULL REFERENCES faturamento.lote(id),
  conta_id uuid NOT NULL UNIQUE REFERENCES faturamento.conta(id),
  sequencial int NOT NULL CHECK (sequencial BETWEEN 1 AND 100),
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  PRIMARY KEY (lote_id, sequencial)
);

-- Demonstrativo de retorno da operadora (um por lote) e glosas por conta.
CREATE TABLE faturamento.demonstrativo (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  lote_id uuid NOT NULL UNIQUE REFERENCES faturamento.lote(id),
  recebido_em date NOT NULL,
  valor_apresentado_centavos bigint NOT NULL CHECK (valor_apresentado_centavos >= 0),
  valor_pago_centavos bigint NOT NULL CHECK (valor_pago_centavos >= 0),
  valor_glosado_centavos bigint NOT NULL CHECK (valor_glosado_centavos >= 0),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK (valor_pago_centavos + valor_glosado_centavos = valor_apresentado_centavos)
);
CREATE TABLE faturamento.glosa (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  conta_id uuid NOT NULL REFERENCES faturamento.conta(id),
  demonstrativo_id uuid NOT NULL REFERENCES faturamento.demonstrativo(id),
  -- Código do motivo de glosa da tabela TISS (dm_tipoGlosa, 4 dígitos).
  codigo_glosa text NOT NULL CHECK (codigo_glosa ~ '^[0-9]{4}$'),
  motivo text NOT NULL CHECK (length(trim(motivo)) BETWEEN 3 AND 500),
  valor_centavos bigint NOT NULL CHECK (valor_centavos > 0),
  prazo_recurso date NOT NULL,
  situacao text NOT NULL DEFAULT 'aberta' CHECK (situacao IN ('aberta','em_recurso')),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
CREATE INDEX glosa_conta_idx ON faturamento.glosa (conta_id);
-- Um recurso por glosa (G2: duas requisições com chaves diferentes não abrem dois recursos).
CREATE TABLE faturamento.recurso (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  glosa_id uuid NOT NULL UNIQUE REFERENCES faturamento.glosa(id),
  justificativa text NOT NULL CHECK (length(trim(justificativa)) BETWEEN 10 AND 500),
  situacao text NOT NULL DEFAULT 'aguardando_envio' CHECK (situacao IN ('aguardando_envio','enviado')),
  chave_envio uuid NOT NULL UNIQUE,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);

-- Nada é apagado no faturamento; o arquivo do lote e a conta que já está em lote não mudam.
CREATE FUNCTION faturamento.proibir_apagar() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'registro do faturamento não é apagado (%.%)', TG_TABLE_SCHEMA, TG_TABLE_NAME;
END$$;
CREATE FUNCTION faturamento.proteger_lote() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF (to_jsonb(NEW) - ARRAY['situacao','protocolo','enviado_em','versao'])
     IS DISTINCT FROM (to_jsonb(OLD) - ARRAY['situacao','protocolo','enviado_em','versao']) THEN
    RAISE EXCEPTION 'arquivo e número do lote TISS são imutáveis';
  END IF;
  RETURN NEW;
END$$;
CREATE FUNCTION faturamento.proteger_conta() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF OLD.lote_id IS NOT NULL AND (to_jsonb(NEW) - ARRAY['situacao','valor_pago_centavos','versao'])
     IS DISTINCT FROM (to_jsonb(OLD) - ARRAY['situacao','valor_pago_centavos','versao']) THEN
    RAISE EXCEPTION 'conta em lote não muda: corrija por recurso de glosa';
  END IF;
  RETURN NEW;
END$$;
CREATE FUNCTION faturamento.proteger_item() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF EXISTS (SELECT 1 FROM faturamento.conta c WHERE c.id = OLD.conta_id AND c.lote_id IS NOT NULL) THEN
    RAISE EXCEPTION 'item de conta em lote não muda';
  END IF;
  IF NEW.tuss IS DISTINCT FROM OLD.tuss OR NEW.quantidade IS DISTINCT FROM OLD.quantidade
     OR NEW.conta_id IS DISTINCT FROM OLD.conta_id OR NEW.sequencial IS DISTINCT FROM OLD.sequencial THEN
    RAISE EXCEPTION 'o faturamento só reaplica preço: o procedimento executado não muda';
  END IF;
  RETURN NEW;
END$$;
CREATE FUNCTION faturamento.somente_insercao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'histórico do faturamento é somente inserção';
END$$;

CREATE TRIGGER lote_protegido BEFORE UPDATE ON faturamento.lote FOR EACH ROW EXECUTE FUNCTION faturamento.proteger_lote();
CREATE TRIGGER conta_protegida BEFORE UPDATE ON faturamento.conta FOR EACH ROW EXECUTE FUNCTION faturamento.proteger_conta();
CREATE TRIGGER item_protegido BEFORE UPDATE ON faturamento.conta_item FOR EACH ROW EXECUTE FUNCTION faturamento.proteger_item();
CREATE TRIGGER historico_imutavel BEFORE UPDATE OR DELETE ON faturamento.conta_historico FOR EACH ROW EXECUTE FUNCTION faturamento.somente_insercao();
CREATE TRIGGER lote_item_imutavel BEFORE UPDATE OR DELETE ON faturamento.lote_item FOR EACH ROW EXECUTE FUNCTION faturamento.somente_insercao();
CREATE TRIGGER preco_imutavel BEFORE UPDATE OR DELETE ON faturamento.preco FOR EACH ROW EXECUTE FUNCTION faturamento.somente_insercao();
CREATE TRIGGER contrato_nao_apaga BEFORE DELETE ON faturamento.contrato FOR EACH ROW EXECUTE FUNCTION faturamento.proibir_apagar();
CREATE TRIGGER tabela_nao_apaga BEFORE DELETE ON faturamento.tabela FOR EACH ROW EXECUTE FUNCTION faturamento.proibir_apagar();
CREATE TRIGGER conta_nao_apaga BEFORE DELETE ON faturamento.conta FOR EACH ROW EXECUTE FUNCTION faturamento.proibir_apagar();
CREATE TRIGGER item_nao_apaga BEFORE DELETE ON faturamento.conta_item FOR EACH ROW EXECUTE FUNCTION faturamento.proibir_apagar();
CREATE TRIGGER pendencia_nao_apaga BEFORE DELETE ON faturamento.pendencia FOR EACH ROW EXECUTE FUNCTION faturamento.proibir_apagar();
CREATE TRIGGER lote_nao_apaga BEFORE DELETE ON faturamento.lote FOR EACH ROW EXECUTE FUNCTION faturamento.proibir_apagar();
CREATE TRIGGER demonstrativo_nao_apaga BEFORE DELETE ON faturamento.demonstrativo FOR EACH ROW EXECUTE FUNCTION faturamento.proibir_apagar();
CREATE TRIGGER glosa_nao_apaga BEFORE DELETE ON faturamento.glosa FOR EACH ROW EXECUTE FUNCTION faturamento.proibir_apagar();
CREATE TRIGGER recurso_nao_apaga BEFORE DELETE ON faturamento.recurso FOR EACH ROW EXECUTE FUNCTION faturamento.proibir_apagar();

-- RLS por unidade e perfil (docs/18 § 4 e § 5): faturista lê e grava; gerente lê (parcial); o núcleo
-- ('sistema') apura as contas a partir do atendimento. Recepção, caixa, médico, financeiro e TI não
-- veem o faturamento de convênios.
DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY['contrato','tabela','preco','lote','conta','conta_item','conta_historico',
                           'pendencia','lote_item','demonstrativo','glosa','recurso'] LOOP
    EXECUTE format('ALTER TABLE faturamento.%I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE faturamento.%I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format($p$CREATE POLICY %I ON faturamento.%I
      USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('faturista','gerente','sistema'))
      WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('faturista','sistema'))$p$,
      t || '_unidade', t);
  END LOOP;
END$$;

GRANT SELECT ON faturamento.contrato, faturamento.tabela, faturamento.preco TO vidacor_app;
-- O número do lote sai do contrato (UPDATE atômico); o resto do contrato muda só por migration/cadastro.
GRANT UPDATE (ultimo_lote, versao) ON faturamento.contrato TO vidacor_app;
GRANT SELECT, INSERT, UPDATE ON faturamento.conta, faturamento.conta_item, faturamento.pendencia,
  faturamento.lote, faturamento.glosa, faturamento.recurso TO vidacor_app;
GRANT SELECT, INSERT ON faturamento.conta_historico, faturamento.lote_item, faturamento.demonstrativo TO vidacor_app;
