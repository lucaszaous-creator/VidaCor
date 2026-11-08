-- 0042 · Financeiro essencial (docs/25 MVP: contas a receber e a pagar, conciliação, repasse;
-- Financeiro.dc.html: "acima de R$ 10 mil, duas aprovações"). Título não se apaga: cancela com motivo.
-- Baixa e conciliação são únicas (G2); quem lança não aprova (docs/18 § 6).

CREATE TABLE financeiro.conta_bancaria (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  nome text NOT NULL CHECK (length(trim(nome)) >= 3),
  saldo_inicial_centavos bigint NOT NULL DEFAULT 0,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);

CREATE TABLE financeiro.titulo (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  tipo text NOT NULL CHECK (tipo IN ('pagar','receber')),
  descricao text NOT NULL CHECK (length(trim(descricao)) >= 3),
  contraparte text NOT NULL CHECK (length(trim(contraparte)) >= 2),
  categoria text NOT NULL CHECK (categoria IN ('fornecedor','folha','repasse','imposto','operadora','particular','outros')),
  valor_centavos bigint NOT NULL CHECK (valor_centavos > 0),
  vencimento date NOT NULL,
  situacao text NOT NULL DEFAULT 'aberto' CHECK (situacao IN ('aberto','pago','cancelado')),
  aprovacoes_exigidas int NOT NULL CHECK (aprovacoes_exigidas BETWEEN 0 AND 2),
  motivo_cancelamento text,
  referencia text,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK (situacao <> 'cancelado' OR length(trim(coalesce(motivo_cancelamento, ''))) >= 5)
);
CREATE INDEX titulo_aberto_idx ON financeiro.titulo (unidade_id, tipo, vencimento) WHERE situacao = 'aberto';
-- Repasse de um médico numa competência vira um título só (G2).
CREATE UNIQUE INDEX titulo_referencia_unica ON financeiro.titulo (unidade_id, referencia) WHERE referencia IS NOT NULL;

CREATE TABLE financeiro.aprovacao (
  titulo_id uuid NOT NULL REFERENCES financeiro.titulo(id),
  aprovado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  criado_em timestamptz NOT NULL DEFAULT now(),
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  correlacao text NOT NULL,
  PRIMARY KEY (titulo_id, aprovado_por)
);

CREATE TABLE financeiro.extrato_linha (
  id uuid PRIMARY KEY,
  conta_id uuid NOT NULL REFERENCES financeiro.conta_bancaria(id),
  id_banco text NOT NULL CHECK (length(id_banco) BETWEEN 1 AND 80),
  data date NOT NULL,
  valor_centavos bigint NOT NULL CHECK (valor_centavos <> 0),
  descricao text NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), correlacao text NOT NULL,
  -- Reimportar o mesmo extrato não duplica (G3).
  UNIQUE (conta_id, id_banco)
);

CREATE TABLE financeiro.baixa (
  id uuid PRIMARY KEY,
  titulo_id uuid NOT NULL UNIQUE REFERENCES financeiro.titulo(id),
  conta_id uuid NOT NULL REFERENCES financeiro.conta_bancaria(id),
  data date NOT NULL,
  extrato_linha_id uuid UNIQUE REFERENCES financeiro.extrato_linha(id),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), correlacao text NOT NULL
);

-- Regra de repasse por médico, versionada (docs/25 § 3.3: "repasse com regra versionada").
CREATE TABLE financeiro.regra_repasse (
  id uuid PRIMARY KEY,
  medico_id uuid NOT NULL REFERENCES identidade.usuario(id),
  versao int NOT NULL CHECK (versao >= 1),
  percentual_pb int NOT NULL CHECK (percentual_pb BETWEEN 1 AND 10000),
  vigente_desde date NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), correlacao text NOT NULL,
  UNIQUE (medico_id, versao)
);

DO $$ DECLARE t text; BEGIN
  FOREACH t IN ARRAY ARRAY['conta_bancaria','titulo','aprovacao','extrato_linha','baixa','regra_repasse'] LOOP
    EXECUTE format('ALTER TABLE financeiro.%I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE financeiro.%I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format('CREATE POLICY %I ON financeiro.%I USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil(''financeiro'',''gerente'',''sistema'')) WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil(''financeiro'',''gerente'',''sistema''))', t || '_unidade', t);
  END LOOP;
END $$;
GRANT SELECT, INSERT, UPDATE ON financeiro.conta_bancaria, financeiro.titulo, financeiro.regra_repasse TO vidacor_app;
GRANT SELECT, INSERT ON financeiro.aprovacao, financeiro.extrato_linha, financeiro.baixa TO vidacor_app;
