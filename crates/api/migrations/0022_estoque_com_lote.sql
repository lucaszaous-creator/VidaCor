-- 0022 · Estoque com lote e validade, começando pelo contraste (Estoque.dc.html; docs/25 critério:
-- "baixa de contraste não deixa saldo negativo com duas baixas simultâneas; recall por lote lista os
-- pacientes"; docs/21: lote de contraste por paciente). Movimento é append-only; o saldo do lote muda
-- só por UPDATE atômico com a condição de saldo (G2).

CREATE SCHEMA estoque;
GRANT USAGE ON SCHEMA estoque TO vidacor_app;

CREATE TABLE estoque.item (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  nome text NOT NULL CHECK (length(trim(nome)) >= 3),
  medida text NOT NULL CHECK (medida IN ('frasco','unidade','kit','caixa','galao','rolo')),
  minimo int NOT NULL DEFAULT 0 CHECK (minimo >= 0),
  local text NOT NULL,
  contraste boolean NOT NULL DEFAULT false,
  ativo boolean NOT NULL DEFAULT true,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);

CREATE TABLE estoque.lote (
  id uuid PRIMARY KEY,
  item_id uuid NOT NULL REFERENCES estoque.item(id),
  numero text NOT NULL CHECK (numero ~ '^[A-Za-z0-9./-]{2,30}$'),
  validade date NOT NULL,
  saldo int NOT NULL CHECK (saldo >= 0),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  UNIQUE (item_id, numero)
);
CREATE INDEX lote_saida_idx ON estoque.lote (item_id, validade) WHERE saldo > 0;

CREATE TABLE estoque.movimento (
  id uuid PRIMARY KEY,
  lote_id uuid NOT NULL REFERENCES estoque.lote(id),
  tipo text NOT NULL CHECK (tipo IN ('entrada','baixa_exame','baixa_manual')),
  quantidade int NOT NULL CHECK (quantidade > 0),
  atendimento_id uuid REFERENCES recepcao.atendimento(id),
  paciente_id uuid REFERENCES identidade.paciente(id),
  motivo text,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK ((tipo = 'baixa_exame') = (atendimento_id IS NOT NULL AND paciente_id IS NOT NULL)),
  CHECK (tipo <> 'baixa_manual' OR length(trim(coalesce(motivo, ''))) >= 5)
);
CREATE INDEX movimento_lote_idx ON estoque.movimento (lote_id, criado_em);
-- Um registro de contraste por atendimento e lote (repetir não baixa duas vezes; G2/G3).
CREATE UNIQUE INDEX movimento_contraste_unico ON estoque.movimento (atendimento_id, lote_id) WHERE tipo = 'baixa_exame';
-- Movimento não se altera nem se apaga (docs/21: eventos imutáveis).
CREATE FUNCTION estoque.movimento_imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'movimento de estoque é imutável'; END $$;
CREATE TRIGGER movimento_sem_alteracao BEFORE UPDATE OR DELETE ON estoque.movimento
  FOR EACH ROW EXECUTE FUNCTION estoque.movimento_imutavel();

ALTER TABLE estoque.item ENABLE ROW LEVEL SECURITY;
ALTER TABLE estoque.item FORCE ROW LEVEL SECURITY;
ALTER TABLE estoque.lote ENABLE ROW LEVEL SECURITY;
ALTER TABLE estoque.lote FORCE ROW LEVEL SECURITY;
ALTER TABLE estoque.movimento ENABLE ROW LEVEL SECURITY;
ALTER TABLE estoque.movimento FORCE ROW LEVEL SECURITY;
CREATE POLICY item_unidade ON estoque.item
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('estoque','gerente','tecnico','enfermagem'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('estoque','gerente'));
CREATE POLICY lote_unidade ON estoque.lote
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('estoque','gerente','tecnico','enfermagem'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('estoque','gerente','tecnico','enfermagem'));
-- Movimento com paciente (rastreio de lote) só para gerência e técnico/enfermagem que registram;
-- o perfil estoque vê o consumo sem identificar o paciente (docs/18 § 4) pela rota, que não junta o paciente.
CREATE POLICY movimento_unidade ON estoque.movimento
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('estoque','gerente','tecnico','enfermagem'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('estoque','gerente','tecnico','enfermagem'));
GRANT SELECT, INSERT, UPDATE ON estoque.item, estoque.lote TO vidacor_app;
GRANT SELECT, INSERT ON estoque.movimento TO vidacor_app;
