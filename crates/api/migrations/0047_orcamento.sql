-- 0047 · Orçamento do particular (Orcamento.dc.html; docs/04 Agenda). O preço vem da tabela vigente e
-- o pacote de uma regra de combinação versionada, sempre no núcleo (G1). Orçamento emitido não muda:
-- cancelar é situação nova com motivo; refazer é outro orçamento (nada é apagado, docs/21).

-- Regra de combinação (pacote) versionada por unidade.
CREATE TABLE financeiro.pacote (
  id uuid PRIMARY KEY, codigo text NOT NULL, nome text NOT NULL CHECK (length(nome) BETWEEN 2 AND 120),
  regra_versao int NOT NULL CHECK (regra_versao >= 1),
  tuss text[] NOT NULL CHECK (cardinality(tuss) BETWEEN 2 AND 10),
  valor_centavos bigint NOT NULL CHECK (valor_centavos > 0),
  vigente_desde date NOT NULL, vigente_ate date,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  UNIQUE (unidade_id, codigo, regra_versao)
);
GRANT SELECT ON financeiro.pacote TO vidacor_app;

CREATE TABLE financeiro.orcamento (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  tabela_versao int NOT NULL,
  pacote_id uuid REFERENCES financeiro.pacote(id),
  subtotal_centavos bigint NOT NULL CHECK (subtotal_centavos > 0),
  pacote_centavos bigint NOT NULL CHECK (pacote_centavos >= 0),
  desconto_centavos bigint NOT NULL CHECK (desconto_centavos >= 0),
  desconto_pontos_base int CHECK (desconto_pontos_base BETWEEN 1 AND 10000),
  desconto_motivo text CHECK (desconto_motivo IN ('paciente_frequente','funcionario_ou_familiar','condicao_social','cobrir_outra_clinica')),
  total_centavos bigint NOT NULL CHECK (total_centavos >= 0),
  pagamento_previsto text NOT NULL CHECK (pagamento_previsto IN ('pix','dinheiro','cartao')),
  valido_ate date NOT NULL,
  situacao text NOT NULL DEFAULT 'emitido' CHECK (situacao IN ('emitido','cancelado')),
  cancelado_por uuid REFERENCES identidade.usuario(id), cancelado_em timestamptz, motivo_cancelamento text,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK (total_centavos = subtotal_centavos - pacote_centavos - desconto_centavos),
  CHECK ((desconto_pontos_base IS NULL) = (desconto_motivo IS NULL)),
  CHECK ((situacao = 'cancelado') = (cancelado_em IS NOT NULL)),
  CHECK (situacao <> 'cancelado' OR length(trim(coalesce(motivo_cancelamento, ''))) >= 5)
);
CREATE INDEX orcamento_paciente_idx ON financeiro.orcamento (paciente_id, criado_em DESC);
CREATE INDEX orcamento_unidade_idx ON financeiro.orcamento (unidade_id, criado_em DESC);

CREATE TABLE financeiro.orcamento_item (
  orcamento_id uuid NOT NULL REFERENCES financeiro.orcamento(id),
  tuss text NOT NULL CHECK (tuss ~ '^[0-9]{8}$'),
  descricao text NOT NULL,
  valor_centavos bigint NOT NULL CHECK (valor_centavos > 0),
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  PRIMARY KEY (orcamento_id, tuss)
);

-- Emitido não muda; só a passagem para cancelado (uma vez) é permitida.
CREATE FUNCTION financeiro.orcamento_sem_alteracao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN RAISE EXCEPTION 'orçamento não se apaga' USING ERRCODE = '42501'; END IF;
  IF OLD.situacao <> 'emitido' OR NEW.situacao <> 'cancelado'
     OR (NEW.id, NEW.codigo, NEW.paciente_id, NEW.total_centavos, NEW.subtotal_centavos, NEW.desconto_centavos, NEW.pacote_centavos, NEW.valido_ate, NEW.criado_por)
        IS DISTINCT FROM (OLD.id, OLD.codigo, OLD.paciente_id, OLD.total_centavos, OLD.subtotal_centavos, OLD.desconto_centavos, OLD.pacote_centavos, OLD.valido_ate, OLD.criado_por) THEN
    RAISE EXCEPTION 'orçamento emitido não se altera; só se cancela com motivo' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER orcamento_imutavel BEFORE UPDATE OR DELETE ON financeiro.orcamento
  FOR EACH ROW EXECUTE FUNCTION financeiro.orcamento_sem_alteracao();
CREATE FUNCTION financeiro.orcamento_item_imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'item de orçamento não se altera' USING ERRCODE = '42501'; END $$;
CREATE TRIGGER orcamento_item_sem_alteracao BEFORE UPDATE OR DELETE ON financeiro.orcamento_item
  FOR EACH ROW EXECUTE FUNCTION financeiro.orcamento_item_imutavel();

ALTER TABLE financeiro.orcamento ENABLE ROW LEVEL SECURITY;
ALTER TABLE financeiro.orcamento FORCE ROW LEVEL SECURITY;
ALTER TABLE financeiro.orcamento_item ENABLE ROW LEVEL SECURITY;
ALTER TABLE financeiro.orcamento_item FORCE ROW LEVEL SECURITY;
-- Recepção emite e lê na unidade; gerência acompanha. O financeiro não vê (tem nome do paciente e exames).
CREATE POLICY orcamento_unidade ON financeiro.orcamento
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente'));
CREATE POLICY orcamento_item_unidade ON financeiro.orcamento_item
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente'));
GRANT SELECT, INSERT ON financeiro.orcamento, financeiro.orcamento_item TO vidacor_app;
GRANT UPDATE (situacao, cancelado_por, cancelado_em, motivo_cancelamento, versao) ON financeiro.orcamento TO vidacor_app;
