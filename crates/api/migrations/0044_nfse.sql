-- 0044 · NFS-e do recebimento do particular (docs/25 MVP: caixa com PIX e NFS-e). Uma nota por
-- recebimento (G2); emissão e cancelamento pelo outbox com chave de idempotência (G3). A descrição é
-- genérica: o nome do exame não vai para a nota (dado de saúde, docs/08).
CREATE TABLE caixa.nota_fiscal (
  id uuid PRIMARY KEY,
  recebimento_id uuid NOT NULL UNIQUE REFERENCES caixa.recebimento(id),
  numero text NOT NULL,
  codigo_verificacao text NOT NULL,
  valor_centavos bigint NOT NULL CHECK (valor_centavos > 0),
  situacao text NOT NULL CHECK (situacao IN ('emitida','cancelada')),
  emitida_em timestamptz NOT NULL DEFAULT now(),
  cancelada_em timestamptz,
  motivo_cancelamento text,
  chave_idempotencia uuid NOT NULL UNIQUE,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK ((situacao = 'cancelada') = (cancelada_em IS NOT NULL))
);
ALTER TABLE caixa.nota_fiscal ENABLE ROW LEVEL SECURITY;
ALTER TABLE caixa.nota_fiscal FORCE ROW LEVEL SECURITY;
CREATE POLICY nota_fiscal_unidade ON caixa.nota_fiscal
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('caixa','gerente','financeiro','sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema'));
GRANT SELECT, INSERT, UPDATE ON caixa.nota_fiscal TO vidacor_app;
