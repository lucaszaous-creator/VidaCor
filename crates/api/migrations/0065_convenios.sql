-- 0065 · Convênios e tabelas (Convenios-Tabelas.dc.html; docs/04 Cadastros base; docs/21 § 3.9;
-- DEC-2026-11-09-110 a 114). Revisão humana obrigatória antes de rodar fora de dev (G6).
--
-- Reaproveita o cadastro do faturamento TISS (0050): o convênio é `faturamento.contrato` e a tabela
-- negociada é `faturamento.tabela` + `faturamento.preco`, versionadas. Esta migration só acrescenta:
-- colunas anuláveis no contrato, histórico imutável do contrato, proposta de tabela com aprovação por
-- segunda pessoa, regras de autorização versionadas, gatilhos de proteção e políticas. Nada é apagado.
-- Antes de rodar em banco com dados, conferir (se houver linhas, a migration falha sem mudar nada e o
-- caso é reportado com proposta de limpeza):
--   SELECT contrato_id, vigente_desde, count(*) FROM faturamento.tabela GROUP BY 1, 2 HAVING count(*) > 1;

-- ---------------------------------------------------------------------------------------------
-- Contrato: termos comerciais e contato da operadora (anuláveis para os contratos já cadastrados).
ALTER TABLE faturamento.contrato
  ADD COLUMN vigencia_inicio date,
  ADD COLUMN vigencia_fim date,
  ADD COLUMN reajuste text CHECK (length(trim(reajuste)) BETWEEN 3 AND 120),
  ADD COLUMN prazo_pagamento_dias int CHECK (prazo_pagamento_dias BETWEEN 1 AND 365),
  ADD COLUMN contato_nome text CHECK (length(trim(contato_nome)) BETWEEN 2 AND 80),
  ADD COLUMN contato_telefone text CHECK (contato_telefone ~ '^[0-9() +-]{8,20}$'),
  ADD COLUMN contato_email text CHECK (length(contato_email) <= 120 AND contato_email ~ '^[^@[:space:]]+@[^@[:space:]]+\.[^@[:space:]]+$'),
  ADD CONSTRAINT contrato_vigencia_coerente
    CHECK (vigencia_inicio IS NULL OR vigencia_fim IS NULL OR vigencia_fim >= vigencia_inicio);

-- Histórico do contrato: cada cadastro e cada mudança, com motivo, antes e depois. Imutável.
CREATE TABLE faturamento.contrato_mudanca (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  contrato_id uuid NOT NULL REFERENCES faturamento.contrato(id),
  tipo text NOT NULL CHECK (tipo IN ('cadastrar','alterar','desativar','reativar')),
  motivo text NOT NULL CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  -- Versão do contrato que esta mudança cria (G2: duas mudanças sobre a mesma versão, só uma entra).
  versao_do_contrato int NOT NULL CHECK (versao_do_contrato >= 1),
  antes jsonb,
  depois jsonb NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  UNIQUE (contrato_id, versao_do_contrato),
  CHECK ((tipo = 'cadastrar') = (antes IS NULL))
);

-- Mudança dos termos pelo papel da aplicação: só com a linha do histórico da mesma versão, gravada na
-- mesma transação. O número do lote (UPDATE atômico do lote) não passa por aqui.
CREATE FUNCTION faturamento.contrato_mudanca_registrada() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF current_user = 'vidacor_app'
     AND (NEW.vigencia_inicio, NEW.vigencia_fim, NEW.reajuste, NEW.prazo_pagamento_dias, NEW.contato_nome,
          NEW.contato_telefone, NEW.contato_email, NEW.prazo_envio_dias, NEW.prazo_recurso_dias, NEW.regra_versao,
          NEW.autorizacao_por_webservice, NEW.ativo, NEW.codigo_prestador, NEW.nome_prestador, NEW.cnes)
         IS DISTINCT FROM
         (OLD.vigencia_inicio, OLD.vigencia_fim, OLD.reajuste, OLD.prazo_pagamento_dias, OLD.contato_nome,
          OLD.contato_telefone, OLD.contato_email, OLD.prazo_envio_dias, OLD.prazo_recurso_dias, OLD.regra_versao,
          OLD.autorizacao_por_webservice, OLD.ativo, OLD.codigo_prestador, OLD.nome_prestador, OLD.cnes)
     AND NOT EXISTS (SELECT 1 FROM faturamento.contrato_mudanca m
                      WHERE m.contrato_id = NEW.id AND m.versao_do_contrato = NEW.versao AND m.criado_em = now()) THEN
    RAISE EXCEPTION 'mudança no contrato sem histórico' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER contrato_mudanca_obrigatoria BEFORE UPDATE ON faturamento.contrato
  FOR EACH ROW EXECUTE FUNCTION faturamento.contrato_mudanca_registrada();

GRANT INSERT ON faturamento.contrato TO vidacor_app;
GRANT UPDATE (vigencia_inicio, vigencia_fim, reajuste, prazo_pagamento_dias, contato_nome, contato_telefone,
  contato_email, prazo_envio_dias, prazo_recurso_dias, regra_versao, autorizacao_por_webservice, ativo,
  codigo_prestador, nome_prestador, cnes) ON faturamento.contrato TO vidacor_app;

-- ---------------------------------------------------------------------------------------------
-- Proposta de tabela nova (reajuste em lote ou preços digitados): o faturamento propõe com os preços
-- já calculados pelo servidor; OUTRA pessoa, da gerência, aprova (ou recusa) com reautenticação. Só a
-- aprovação cria a versão em `faturamento.tabela`, que o faturamento usa pela data do atendimento.
CREATE TABLE faturamento.tabela_proposta (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  contrato_id uuid NOT NULL REFERENCES faturamento.contrato(id),
  tipo text NOT NULL CHECK (tipo IN ('reajuste','precos')),
  reajuste_pontos_base int CHECK (reajuste_pontos_base BETWEEN 1 AND 3000),
  -- Versão de partida (a vigente quando a proposta foi feita), para conferir o reajuste.
  base_tabela_id uuid REFERENCES faturamento.tabela(id),
  vigente_desde date NOT NULL,
  descricao text NOT NULL CHECK (length(trim(descricao)) BETWEEN 3 AND 120),
  documento text NOT NULL CHECK (length(trim(documento)) BETWEEN 3 AND 120),
  motivo text NOT NULL CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  situacao text NOT NULL DEFAULT 'pendente' CHECK (situacao IN ('pendente','aprovada','recusada')),
  decidido_por uuid REFERENCES identidade.usuario(id),
  decidido_em timestamptz,
  motivo_decisao text CHECK (length(trim(motivo_decisao)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK ((tipo = 'reajuste') = (reajuste_pontos_base IS NOT NULL AND base_tabela_id IS NOT NULL)),
  CHECK ((situacao = 'pendente') = (decidido_por IS NULL AND decidido_em IS NULL)),
  CHECK (situacao <> 'recusada' OR motivo_decisao IS NOT NULL),
  -- Separação de funções no próprio banco: quem propõe não decide.
  CHECK (decidido_por IS NULL OR decidido_por <> criado_por)
);
-- Uma proposta pendente por contrato (G2).
CREATE UNIQUE INDEX tabela_proposta_uma_pendente ON faturamento.tabela_proposta (contrato_id)
  WHERE situacao = 'pendente';

CREATE TABLE faturamento.tabela_proposta_preco (
  proposta_id uuid NOT NULL REFERENCES faturamento.tabela_proposta(id),
  -- Como em `faturamento.preco`, sem chave estrangeira: o reajuste leva a tabela negociada inteira.
  tuss text NOT NULL CHECK (tuss ~ '^[0-9]{8}$'),
  descricao text NOT NULL CHECK (length(trim(descricao)) BETWEEN 1 AND 150),
  valor_centavos bigint NOT NULL CHECK (valor_centavos > 0 AND valor_centavos <= 10000000),
  -- Preço na versão de partida (nulo quando o exame entra agora).
  valor_anterior_centavos bigint CHECK (valor_anterior_centavos > 0),
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  PRIMARY KEY (proposta_id, tuss)
);

-- A proposta só muda uma vez, de pendente para aprovada ou recusada; os preços nunca mudam.
CREATE FUNCTION faturamento.tabela_proposta_decisao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN RAISE EXCEPTION 'proposta de tabela não se apaga' USING ERRCODE = '42501'; END IF;
  IF OLD.situacao <> 'pendente' OR NEW.situacao = 'pendente'
     OR (NEW.id, NEW.codigo, NEW.contrato_id, NEW.tipo, NEW.reajuste_pontos_base, NEW.base_tabela_id, NEW.vigente_desde,
         NEW.descricao, NEW.documento, NEW.motivo, NEW.criado_por, NEW.criado_em, NEW.unidade_id)
        IS DISTINCT FROM
        (OLD.id, OLD.codigo, OLD.contrato_id, OLD.tipo, OLD.reajuste_pontos_base, OLD.base_tabela_id, OLD.vigente_desde,
         OLD.descricao, OLD.documento, OLD.motivo, OLD.criado_por, OLD.criado_em, OLD.unidade_id) THEN
    RAISE EXCEPTION 'proposta de tabela só muda de pendente para aprovada ou recusada' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER tabela_proposta_imutavel BEFORE UPDATE OR DELETE ON faturamento.tabela_proposta
  FOR EACH ROW EXECUTE FUNCTION faturamento.tabela_proposta_decisao();
CREATE TRIGGER tabela_proposta_preco_imutavel BEFORE UPDATE OR DELETE ON faturamento.tabela_proposta_preco
  FOR EACH ROW EXECUTE FUNCTION faturamento.somente_insercao();

-- A versão da tabela aponta para a proposta aprovada que a criou (nulo nas versões anteriores a 0065).
ALTER TABLE faturamento.tabela
  ADD COLUMN proposta_id uuid UNIQUE REFERENCES faturamento.tabela_proposta(id),
  ADD COLUMN motivo text CHECK (length(trim(motivo)) BETWEEN 5 AND 300);
-- Duas versões do mesmo contrato não começam no mesmo dia (G2).
ALTER TABLE faturamento.tabela
  ADD CONSTRAINT tabela_uma_por_vigencia UNIQUE (contrato_id, vigente_desde);

-- Pelo papel da aplicação, versão de tabela só nasce de proposta aprovada AGORA (mesma transação), do
-- mesmo contrato e vigência; e os preços só entram na versão criada agora.
CREATE FUNCTION faturamento.tabela_escrita_permitida() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF current_user = 'vidacor_app' AND NOT EXISTS (
       SELECT 1 FROM faturamento.tabela_proposta p
        WHERE p.id = NEW.proposta_id AND p.situacao = 'aprovada' AND p.decidido_em = now()
          AND p.contrato_id = NEW.contrato_id AND p.vigente_desde = NEW.vigente_desde
          AND p.unidade_id = NEW.unidade_id) THEN
    RAISE EXCEPTION 'versão de tabela só nasce de proposta aprovada' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER tabela_escrita BEFORE INSERT ON faturamento.tabela
  FOR EACH ROW EXECUTE FUNCTION faturamento.tabela_escrita_permitida();

CREATE FUNCTION faturamento.preco_escrita_permitida() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF current_user = 'vidacor_app' AND NOT EXISTS (
       SELECT 1 FROM faturamento.tabela t
        WHERE t.id = NEW.tabela_id AND t.criado_em = now() AND t.proposta_id IS NOT NULL
          AND t.unidade_id = NEW.unidade_id) THEN
    RAISE EXCEPTION 'preço só entra na versão criada agora' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER preco_escrita BEFORE INSERT ON faturamento.preco
  FOR EACH ROW EXECUTE FUNCTION faturamento.preco_escrita_permitida();

-- ---------------------------------------------------------------------------------------------
-- Regras de autorização da operadora por exame (senha, validade, documentos), versão inteira nova a
-- cada mudança, com motivo. Imutáveis.
CREATE TABLE faturamento.regra_autorizacao (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  contrato_id uuid NOT NULL REFERENCES faturamento.contrato(id),
  versao_regras int NOT NULL CHECK (versao_regras >= 1),
  motivo text NOT NULL CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  UNIQUE (contrato_id, versao_regras)
);
CREATE TABLE faturamento.regra_autorizacao_item (
  regra_id uuid NOT NULL REFERENCES faturamento.regra_autorizacao(id),
  tuss text NOT NULL REFERENCES agenda.procedimento(tuss),
  exige_senha boolean NOT NULL,
  validade_dias int CHECK (validade_dias BETWEEN 1 AND 365),
  documentos text NOT NULL CHECK (length(trim(documentos)) BETWEEN 3 AND 200),
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  PRIMARY KEY (regra_id, tuss),
  CHECK (exige_senha = (validade_dias IS NOT NULL))
);
CREATE TRIGGER regra_autorizacao_imutavel BEFORE UPDATE OR DELETE ON faturamento.regra_autorizacao
  FOR EACH ROW EXECUTE FUNCTION faturamento.somente_insercao();
CREATE TRIGGER regra_autorizacao_item_imutavel BEFORE UPDATE OR DELETE ON faturamento.regra_autorizacao_item
  FOR EACH ROW EXECUTE FUNCTION faturamento.somente_insercao();
CREATE TRIGGER contrato_mudanca_imutavel BEFORE UPDATE OR DELETE ON faturamento.contrato_mudanca
  FOR EACH ROW EXECUTE FUNCTION faturamento.somente_insercao();

-- ---------------------------------------------------------------------------------------------
-- RLS: mesma regra do faturamento (faturista e gerência leem; faturista grava) e, a mais, a gerência
-- decide a proposta e cria a versão aprovada da tabela (políticas permissivas somam).
DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY['contrato_mudanca','tabela_proposta_preco','regra_autorizacao',
                           'regra_autorizacao_item'] LOOP
    EXECUTE format('ALTER TABLE faturamento.%I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE faturamento.%I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format($p$CREATE POLICY %I ON faturamento.%I
      USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('faturista','gerente'))
      WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('faturista'))$p$,
      t || '_unidade', t);
  END LOOP;
END $$;
-- Proposta: faturista e gerência leem; só o faturista propõe; só a gerência decide.
ALTER TABLE faturamento.tabela_proposta ENABLE ROW LEVEL SECURITY;
ALTER TABLE faturamento.tabela_proposta FORCE ROW LEVEL SECURITY;
CREATE POLICY tabela_proposta_leitura ON faturamento.tabela_proposta FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('faturista','gerente'));
CREATE POLICY tabela_proposta_pedido ON faturamento.tabela_proposta FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('faturista'));
CREATE POLICY tabela_proposta_decisao ON faturamento.tabela_proposta FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
CREATE POLICY tabela_aprovada ON faturamento.tabela FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
CREATE POLICY preco_aprovado ON faturamento.preco FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));

GRANT SELECT, INSERT ON faturamento.contrato_mudanca, faturamento.tabela_proposta_preco,
  faturamento.regra_autorizacao, faturamento.regra_autorizacao_item TO vidacor_app;
GRANT SELECT, INSERT ON faturamento.tabela_proposta TO vidacor_app;
GRANT UPDATE (situacao, decidido_por, decidido_em, motivo_decisao, versao) ON faturamento.tabela_proposta TO vidacor_app;
GRANT INSERT ON faturamento.tabela, faturamento.preco TO vidacor_app;
