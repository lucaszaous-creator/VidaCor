-- 0064 · Exames e procedimentos e tabela de preços versionada (Exames-Procedimentos.dc.html;
-- docs/04 Cadastros base; DEC-2026-11-08-93 e 94). Revisão humana obrigatória antes de rodar fora de
-- dev (G6). Só acrescenta: tabela de histórico, gatilhos de proteção, uma coluna anulável, uma chave
-- estrangeira e uma restrição única. Nada é apagado. Antes de rodar em banco com dados, conferir:
--   SELECT unidade_id, vigente_desde, count(*) FROM financeiro.tabela_preco GROUP BY 1, 2 HAVING count(*) > 1;
--   SELECT DISTINCT tuss FROM financeiro.preco WHERE tuss NOT IN (SELECT tuss FROM agenda.procedimento);
-- Se houver linhas, a migration falha sem mudar nada; o caso é reportado com proposta de limpeza.

-- ---------------------------------------------------------------------------------------------
-- Catálogo (agenda.procedimento, único da clínica): incluir, mudar duração, modalidade e ativo,
-- sempre com motivo e histórico imutável. Procedimento nunca se apaga (desativa com motivo).
CREATE TABLE agenda.procedimento_mudanca (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  tuss text NOT NULL REFERENCES agenda.procedimento(tuss),
  tipo text NOT NULL CHECK (tipo IN ('incluir','alterar','desativar','reativar')),
  motivo text NOT NULL CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  versao_do_procedimento int NOT NULL CHECK (versao_do_procedimento >= 1),
  antes jsonb,
  depois jsonb NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  -- Uma mudança por versão do procedimento (G2: duas mudanças sobre a mesma versão, só uma entra).
  UNIQUE (tuss, versao_do_procedimento),
  CHECK ((tipo = 'incluir') = (antes IS NULL))
);
CREATE FUNCTION agenda.procedimento_mudanca_imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'histórico do catálogo é imutável' USING ERRCODE = '42501'; END $$;
CREATE TRIGGER procedimento_mudanca_sem_alteracao BEFORE UPDATE OR DELETE ON agenda.procedimento_mudanca
  FOR EACH ROW EXECUTE FUNCTION agenda.procedimento_mudanca_imutavel();
ALTER TABLE agenda.procedimento_mudanca ENABLE ROW LEVEL SECURITY;
ALTER TABLE agenda.procedimento_mudanca FORCE ROW LEVEL SECURITY;
-- O catálogo é da clínica: o histórico é lido pela recepção e gerência de qualquer unidade; só a
-- gerência grava, e grava em nome da própria unidade.
CREATE POLICY procedimento_mudanca_leitura ON agenda.procedimento_mudanca FOR SELECT
  USING (comum.ctx_tem_perfil('recepcao','gerente'));
CREATE POLICY procedimento_mudanca_escrita ON agenda.procedimento_mudanca FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
GRANT SELECT, INSERT ON agenda.procedimento_mudanca TO vidacor_app;

-- Escrita no catálogo pelo papel da aplicação: só a gerência; nunca se apaga; o código TUSS, o nome
-- e o contraste não mudam por aqui (colunas fora do GRANT).
CREATE FUNCTION agenda.procedimento_escrita_permitida() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'procedimento não se apaga; desative com motivo' USING ERRCODE = '42501';
  END IF;
  IF current_user = 'vidacor_app' AND NOT comum.ctx_tem_perfil('gerente') THEN
    RAISE EXCEPTION 'escrita em agenda.procedimento negada' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER procedimento_escrita BEFORE INSERT OR UPDATE OR DELETE ON agenda.procedimento
  FOR EACH ROW EXECUTE FUNCTION agenda.procedimento_escrita_permitida();
GRANT INSERT ON agenda.procedimento TO vidacor_app;
GRANT UPDATE (duracao_min, modalidade, ativo, versao) ON agenda.procedimento TO vidacor_app;

-- ---------------------------------------------------------------------------------------------
-- Tabela de preços: cada versão é nova, com data de vigência própria; a vigente nunca é alterada.
ALTER TABLE financeiro.tabela_preco
  ADD COLUMN motivo text CHECK (length(trim(motivo)) BETWEEN 5 AND 300);
-- Duas versões da mesma unidade não começam no mesmo dia (G2).
ALTER TABLE financeiro.tabela_preco
  ADD CONSTRAINT tabela_preco_uma_por_vigencia UNIQUE (unidade_id, vigente_desde);
-- Preço só de procedimento do catálogo.
ALTER TABLE financeiro.preco
  ADD CONSTRAINT preco_procedimento_fk FOREIGN KEY (tuss) REFERENCES agenda.procedimento(tuss);

CREATE FUNCTION financeiro.preco_imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION '% é versionada: a correção é uma versão nova da tabela', TG_TABLE_NAME USING ERRCODE = '42501';
END $$;
CREATE TRIGGER tabela_preco_imutavel BEFORE UPDATE OR DELETE ON financeiro.tabela_preco
  FOR EACH ROW EXECUTE FUNCTION financeiro.preco_imutavel();
CREATE TRIGGER preco_imutavel BEFORE UPDATE OR DELETE ON financeiro.preco
  FOR EACH ROW EXECUTE FUNCTION financeiro.preco_imutavel();

-- Pelo papel da aplicação: a versão nova é da gerência da própria unidade, com motivo; os preços só
-- entram junto com a versão, na mesma transação (criada agora), e nunca numa versão antiga.
CREATE FUNCTION financeiro.tabela_preco_escrita_permitida() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF current_user = 'vidacor_app' AND NOT (comum.ctx_tem_perfil('gerente')
       AND NEW.unidade_id = comum.ctx_unidade() AND NEW.motivo IS NOT NULL) THEN
    RAISE EXCEPTION 'escrita em financeiro.tabela_preco negada' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER tabela_preco_escrita BEFORE INSERT ON financeiro.tabela_preco
  FOR EACH ROW EXECUTE FUNCTION financeiro.tabela_preco_escrita_permitida();

CREATE FUNCTION financeiro.preco_escrita_permitida() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF current_user = 'vidacor_app' AND NOT (comum.ctx_tem_perfil('gerente') AND EXISTS (
       SELECT 1 FROM financeiro.tabela_preco t
        WHERE t.id = NEW.tabela_id AND t.unidade_id = comum.ctx_unidade() AND t.criado_em = now())) THEN
    RAISE EXCEPTION 'escrita em financeiro.preco negada' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER preco_escrita BEFORE INSERT ON financeiro.preco
  FOR EACH ROW EXECUTE FUNCTION financeiro.preco_escrita_permitida();
GRANT INSERT ON financeiro.tabela_preco, financeiro.preco TO vidacor_app;
