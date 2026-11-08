-- 0001 · Base: extensões, papéis, contadores, lastro e integração (docs/21, docs/18 § 7, G2, G3)
-- Revisão humana obrigatória antes de rodar em qualquer ambiente fora de dev (G6).

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Papel da aplicação: sem BYPASSRLS, sem superusuário, não é dono das tabelas (docs/18 § 7).
-- A senha é definida fora da migration (instalador, compose ou teste), nunca no repositório.
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'vidacor_app') THEN
    CREATE ROLE vidacor_app LOGIN NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE;
  END IF;
END$$;

CREATE SCHEMA comum;
CREATE SCHEMA identidade;
CREATE SCHEMA agenda;
CREATE SCHEMA recepcao;
CREATE SCHEMA exames;
CREATE SCHEMA lastro;
CREATE SCHEMA integracao;

GRANT USAGE ON SCHEMA comum, identidade, agenda, recepcao, exames, lastro, integracao TO vidacor_app;

-- Contadores de código legível por prefixo e ano. Incremento atômico na mesma transação do
-- registro (G2, operação atômica): sem buraco por corrida, rollback desfaz o número.
CREATE TABLE comum.contador (
  prefixo text NOT NULL CHECK (prefixo ~ '^[A-Z]{3}$'),
  ano     int  NOT NULL CHECK (ano BETWEEN 2000 AND 2999),
  valor   bigint NOT NULL DEFAULT 0 CHECK (valor >= 0),
  PRIMARY KEY (prefixo, ano)
);
GRANT SELECT, INSERT, UPDATE ON comum.contador TO vidacor_app;

CREATE FUNCTION comum.proximo_numero(p_prefixo text, p_ano int) RETURNS bigint
LANGUAGE sql AS $$
  INSERT INTO comum.contador AS c (prefixo, ano, valor) VALUES (p_prefixo, p_ano, 1)
  ON CONFLICT (prefixo, ano) DO UPDATE SET valor = c.valor + 1
  RETURNING valor
$$;

-- Eventos imutáveis com hash encadeado (docs/21 § 5 itens 2 e 9).
CREATE TABLE lastro.evento (
  id             uuid PRIMARY KEY,
  sequencia      bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
  dominio        text NOT NULL,
  tipo           text NOT NULL,
  registro_id    uuid NOT NULL,
  registro_codigo text,
  dados          jsonb NOT NULL,
  criado_em      timestamptz NOT NULL DEFAULT now(),
  criado_por     uuid,
  origem         text NOT NULL,
  estacao        text,
  unidade_id     uuid NOT NULL,
  versao         int  NOT NULL DEFAULT 1,
  correlacao     text NOT NULL,
  hash_anterior  bytea,
  hash           bytea NOT NULL
);
CREATE INDEX evento_registro_idx ON lastro.evento (registro_id);

-- SECURITY DEFINER: o encadeamento precisa ver o último evento de todas as unidades, acima do RLS.
-- A função não recebe nada do usuário além da própria linha inserida.
CREATE FUNCTION lastro.encadear() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE anterior bytea;
BEGIN
  -- Serializa o encadeamento: um evento por vez recebe o hash do anterior.
  PERFORM pg_advisory_xact_lock(hashtext('lastro.evento'));
  SELECT e.hash INTO anterior FROM lastro.evento e ORDER BY e.sequencia DESC LIMIT 1;
  NEW.hash_anterior := anterior;
  NEW.hash := digest(
    coalesce(encode(anterior, 'hex'), '') || '|' || NEW.id::text || '|' || NEW.dominio || '|' ||
    NEW.tipo || '|' || NEW.registro_id::text || '|' || NEW.dados::text || '|' ||
    to_char(NEW.criado_em AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US'),
    'sha256');
  RETURN NEW;
END$$;

CREATE TRIGGER evento_encadear BEFORE INSERT ON lastro.evento
  FOR EACH ROW EXECUTE FUNCTION lastro.encadear();

-- Append-only: a aplicação só insere e lê. Nenhuma política ou permissão de UPDATE/DELETE.
GRANT SELECT, INSERT ON lastro.evento TO vidacor_app;

CREATE FUNCTION lastro.proibir_alteracao() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'lastro.evento é somente inserção (docs/21 § 5)';
END$$;
CREATE TRIGGER evento_imutavel BEFORE UPDATE OR DELETE ON lastro.evento
  FOR EACH ROW EXECUTE FUNCTION lastro.proibir_alteracao();

-- Idempotência das rotas de escrita (G3): a mesma chave do mesmo usuário na mesma rota devolve
-- a mesma resposta. A chave é UUID gerado pela tela, nunca dado pessoal.
CREATE TABLE integracao.idempotencia (
  usuario_id  uuid NOT NULL,
  rota        text NOT NULL,
  chave       uuid NOT NULL,
  status      int,
  resposta    jsonb,
  criado_em   timestamptz NOT NULL DEFAULT now(),
  concluido_em timestamptz,
  PRIMARY KEY (usuario_id, rota, chave)
);
GRANT SELECT, INSERT, UPDATE ON integracao.idempotencia TO vidacor_app;

-- Outbox transacional (docs/11 § 6, ADR-0007): gravado na mesma transação do negócio.
CREATE TABLE integracao.outbox (
  id               uuid PRIMARY KEY,
  tipo             text NOT NULL,
  chave_idempotencia uuid NOT NULL UNIQUE,
  carga            jsonb NOT NULL,
  unidade_id       uuid NOT NULL,
  correlacao       text NOT NULL,
  criado_em        timestamptz NOT NULL DEFAULT now(),
  tentativas       int NOT NULL DEFAULT 0,
  proximo_em       timestamptz NOT NULL DEFAULT now(),
  concluido_em     timestamptz,
  ultimo_erro      text
);
CREATE INDEX outbox_pendente_idx ON integracao.outbox (proximo_em) WHERE concluido_em IS NULL;
GRANT SELECT, INSERT, UPDATE ON integracao.outbox TO vidacor_app;
