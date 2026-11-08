-- 0015 · Caixa do particular, tabela de preços versionada, desconto com autorização, estorno com
-- separação de funções e eventos de webhook recebidos (docs/27 FC-CXA, FC-PIX; G1, G2, G3).

CREATE SCHEMA financeiro;
CREATE SCHEMA caixa;
GRANT USAGE ON SCHEMA financeiro, caixa TO vidacor_app;

CREATE TABLE financeiro.tabela_preco (
  id uuid PRIMARY KEY, versao int NOT NULL CHECK (versao >= 1), vigente_desde date NOT NULL,
  descricao text NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  correlacao text NOT NULL,
  UNIQUE (unidade_id, versao)
);
CREATE TABLE financeiro.preco (
  tabela_id uuid NOT NULL REFERENCES financeiro.tabela_preco(id),
  tuss text NOT NULL CHECK (tuss ~ '^[0-9]{8}$'),
  valor_centavos bigint NOT NULL CHECK (valor_centavos > 0),
  PRIMARY KEY (tabela_id, tuss)
);
GRANT SELECT ON financeiro.tabela_preco, financeiro.preco TO vidacor_app;

CREATE TABLE caixa.caixa (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  operador_id uuid NOT NULL REFERENCES identidade.usuario(id),
  fundo_centavos bigint NOT NULL CHECK (fundo_centavos >= 0),
  aberto_em timestamptz NOT NULL DEFAULT now(),
  fechado_em timestamptz, esperado_centavos bigint, contado_centavos bigint, diferenca_centavos bigint,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK ((fechado_em IS NULL) = (contado_centavos IS NULL))
);
-- Um caixa aberto por operador (G2).
CREATE UNIQUE INDEX caixa_um_aberto ON caixa.caixa (operador_id) WHERE fechado_em IS NULL;

CREATE TABLE caixa.autorizacao_desconto (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  pontos_base int NOT NULL CHECK (pontos_base BETWEEN 1 AND 10000),
  motivo text NOT NULL CHECK (length(trim(motivo)) >= 5),
  solicitante_id uuid NOT NULL REFERENCES identidade.usuario(id),
  autorizado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  valida_ate timestamptz NOT NULL,
  usada_em timestamptz,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK (autorizado_por <> solicitante_id)
);

CREATE TABLE caixa.recebimento (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  caixa_id uuid NOT NULL REFERENCES caixa.caixa(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  atendimento_id uuid REFERENCES recepcao.atendimento(id),
  subtotal_centavos bigint NOT NULL CHECK (subtotal_centavos > 0),
  desconto_centavos bigint NOT NULL DEFAULT 0 CHECK (desconto_centavos >= 0),
  desconto_pontos_base int NOT NULL DEFAULT 0,
  desconto_motivo text,
  desconto_aplicado_por uuid,
  autorizacao_id uuid UNIQUE REFERENCES caixa.autorizacao_desconto(id),
  total_centavos bigint NOT NULL CHECK (total_centavos > 0),
  tabela_versao int NOT NULL,
  forma text NOT NULL CHECK (forma IN ('pix','dinheiro','cartao')),
  situacao text NOT NULL CHECK (situacao IN ('aguardando_pagamento','pago','estornado')),
  pix_txid text UNIQUE, pix_copia_e_cola text, pix_end_to_end text UNIQUE,
  chave_psp uuid NOT NULL UNIQUE,
  pago_em timestamptz,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK (total_centavos = subtotal_centavos - desconto_centavos),
  CHECK (desconto_centavos = 0 OR desconto_motivo IS NOT NULL)
);
CREATE INDEX recebimento_caixa_idx ON caixa.recebimento (caixa_id);
CREATE TABLE caixa.recebimento_item (
  recebimento_id uuid NOT NULL REFERENCES caixa.recebimento(id),
  tuss text NOT NULL, descricao text NOT NULL,
  valor_centavos bigint NOT NULL CHECK (valor_centavos > 0),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL DEFAULT 'caixa',
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  PRIMARY KEY (recebimento_id, tuss)
);

CREATE TABLE caixa.estorno (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  recebimento_id uuid NOT NULL REFERENCES caixa.recebimento(id),
  motivo text NOT NULL CHECK (length(trim(motivo)) >= 5),
  pedido_por uuid NOT NULL REFERENCES identidade.usuario(id),
  situacao text NOT NULL DEFAULT 'pedido' CHECK (situacao IN ('pedido','aprovado')),
  aprovado_por uuid REFERENCES identidade.usuario(id), aprovado_em timestamptz,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  -- Separação de funções (docs/18 § 6 item 3): quem pede não aprova.
  CHECK (aprovado_por IS NULL OR aprovado_por <> pedido_por)
);
-- Um estorno por recebimento (G2).
CREATE UNIQUE INDEX estorno_um_por_recebimento ON caixa.estorno (recebimento_id);

-- RLS: caixa vê só o próprio caixa; gerente e financeiro, a unidade; o núcleo (sistema) processa o PIX.
CREATE FUNCTION comum.ctx_usuario() RETURNS uuid LANGUAGE sql STABLE AS
$$ SELECT NULLIF(current_setting('app.usuario_id', true), '')::uuid $$;
GRANT EXECUTE ON FUNCTION comum.ctx_usuario() TO vidacor_app;

ALTER TABLE caixa.caixa ENABLE ROW LEVEL SECURITY;
ALTER TABLE caixa.caixa FORCE ROW LEVEL SECURITY;
CREATE POLICY caixa_acesso ON caixa.caixa
  USING (unidade_id = comum.ctx_unidade() AND (operador_id = comum.ctx_usuario()
         OR comum.ctx_tem_perfil('gerente','financeiro','sistema')))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND (operador_id = comum.ctx_usuario() OR comum.ctx_tem_perfil('sistema')));

ALTER TABLE caixa.recebimento ENABLE ROW LEVEL SECURITY;
ALTER TABLE caixa.recebimento FORCE ROW LEVEL SECURITY;
CREATE POLICY recebimento_acesso ON caixa.recebimento
  USING (unidade_id = comum.ctx_unidade() AND (
    EXISTS (SELECT 1 FROM caixa.caixa c WHERE c.id = caixa_id)
    OR comum.ctx_tem_perfil('gerente','financeiro','sistema')))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('caixa','gerente','financeiro','sistema'));

ALTER TABLE caixa.recebimento_item ENABLE ROW LEVEL SECURITY;
ALTER TABLE caixa.recebimento_item FORCE ROW LEVEL SECURITY;
CREATE POLICY recebimento_item_acesso ON caixa.recebimento_item
  USING (EXISTS (SELECT 1 FROM caixa.recebimento r WHERE r.id = recebimento_id))
  WITH CHECK (unidade_id = comum.ctx_unidade());

ALTER TABLE caixa.estorno ENABLE ROW LEVEL SECURITY;
ALTER TABLE caixa.estorno FORCE ROW LEVEL SECURITY;
CREATE POLICY estorno_acesso ON caixa.estorno
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('caixa','gerente','financeiro','sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade());

ALTER TABLE caixa.autorizacao_desconto ENABLE ROW LEVEL SECURITY;
ALTER TABLE caixa.autorizacao_desconto FORCE ROW LEVEL SECURITY;
CREATE POLICY autorizacao_acesso ON caixa.autorizacao_desconto
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('caixa','gerente','financeiro'))
  WITH CHECK (unidade_id = comum.ctx_unidade());

GRANT SELECT, INSERT, UPDATE ON caixa.caixa, caixa.recebimento, caixa.recebimento_item, caixa.estorno,
  caixa.autorizacao_desconto TO vidacor_app;

-- Caixa vê o cadastro (N1) do paciente e o atendimento (sem dado clínico), não a triagem (docs/18 § 4).
CREATE POLICY paciente_caixa ON identidade.paciente FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('caixa','gerente','financeiro'));

-- Eventos de webhook recebidos: o ID do evento é gravado ANTES de agir; repetido para (G3).
CREATE TABLE integracao.evento_recebido (
  provedor text NOT NULL,
  evento_id text NOT NULL CHECK (length(evento_id) BETWEEN 1 AND 100),
  recebido_em timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (provedor, evento_id)
);
GRANT SELECT, INSERT ON integracao.evento_recebido TO vidacor_app;
