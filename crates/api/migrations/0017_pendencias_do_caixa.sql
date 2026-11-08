-- 0017 · Pendências da gerência no caixa: pedido de desconto acima do limite (fila em vez de link no
-- chat, mockup Caixa "aguardando a autorização") e recusa de estorno com motivo. Nada é apagado:
-- pedido decidido muda de situação e fica no lastro (docs/21). G2: um pedido aberto por paciente e
-- caixa; um estorno vivo por recebimento.

CREATE TABLE caixa.pedido_desconto (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  pontos_base int NOT NULL CHECK (pontos_base BETWEEN 1 AND 10000),
  motivo text NOT NULL CHECK (length(trim(motivo)) >= 5),
  solicitante_id uuid NOT NULL REFERENCES identidade.usuario(id),
  situacao text NOT NULL DEFAULT 'pedido' CHECK (situacao IN ('pedido','autorizado','recusado','cancelado')),
  autorizacao_id uuid UNIQUE REFERENCES caixa.autorizacao_desconto(id),
  decidido_por uuid REFERENCES identidade.usuario(id),
  decidido_em timestamptz,
  motivo_decisao text,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK ((situacao = 'pedido') = (decidido_em IS NULL)),
  CHECK (situacao <> 'autorizado' OR autorizacao_id IS NOT NULL),
  CHECK (situacao <> 'recusado' OR length(trim(coalesce(motivo_decisao, ''))) >= 5),
  -- Separação de funções (docs/18 § 6): quem pede não autoriza nem recusa; só cancela.
  CHECK (decidido_por IS NULL OR situacao = 'cancelado' OR decidido_por <> solicitante_id)
);
CREATE UNIQUE INDEX pedido_desconto_um_aberto ON caixa.pedido_desconto (paciente_id, solicitante_id) WHERE situacao = 'pedido';
CREATE INDEX pedido_desconto_situacao_idx ON caixa.pedido_desconto (unidade_id, situacao);

ALTER TABLE caixa.pedido_desconto ENABLE ROW LEVEL SECURITY;
ALTER TABLE caixa.pedido_desconto FORCE ROW LEVEL SECURITY;
-- Caixa vê e altera só os próprios pedidos; gerência e financeiro, os da unidade.
CREATE POLICY pedido_desconto_acesso ON caixa.pedido_desconto
  USING (unidade_id = comum.ctx_unidade() AND (solicitante_id = comum.ctx_usuario()
         OR comum.ctx_tem_perfil('gerente','financeiro')))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND (solicitante_id = comum.ctx_usuario()
         OR comum.ctx_tem_perfil('gerente')));
GRANT SELECT, INSERT, UPDATE ON caixa.pedido_desconto TO vidacor_app;

-- Estorno recusado com motivo; depois da recusa o caixa pode pedir de novo (um estorno vivo por vez).
ALTER TABLE caixa.estorno DROP CONSTRAINT estorno_situacao_check;
ALTER TABLE caixa.estorno ADD CONSTRAINT estorno_situacao_check CHECK (situacao IN ('pedido','aprovado','recusado'));
ALTER TABLE caixa.estorno
  ADD COLUMN recusado_por uuid REFERENCES identidade.usuario(id),
  ADD COLUMN recusado_em timestamptz,
  ADD COLUMN motivo_recusa text,
  ADD CONSTRAINT estorno_recusa_coerente CHECK ((situacao = 'recusado') = (recusado_em IS NOT NULL)
    AND (situacao <> 'recusado' OR (length(trim(coalesce(motivo_recusa, ''))) >= 5 AND recusado_por <> pedido_por)));
DROP INDEX caixa.estorno_um_por_recebimento;
CREATE UNIQUE INDEX estorno_um_vivo_por_recebimento ON caixa.estorno (recebimento_id) WHERE situacao <> 'recusado';
