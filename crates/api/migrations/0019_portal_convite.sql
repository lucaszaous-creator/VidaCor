-- 0019 · Convite e login do portal do paciente (docs/25 Fase 1, SEC-AUT-6, SEC-DIS-5, docs/15 G2).
-- Convite: token aleatório de uso único (só o hash no banco), vale 72 h, um vivo por paciente; o aceite
-- confere o nascimento (segundo identificador) e é atômico. Senha só como hash argon2id. Tentativas
-- erradas contam e bloqueiam por tempo; as mensagens não revelam se a conta existe.

CREATE TABLE portal.convite (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  token_hash bytea NOT NULL UNIQUE,
  expira_em timestamptz NOT NULL,
  usado_em timestamptz,
  revogado_em timestamptz,
  motivo_revogacao text,
  tentativas int NOT NULL DEFAULT 0 CHECK (tentativas >= 0),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK (usado_em IS NULL OR revogado_em IS NULL)
);
-- Um convite vivo por paciente (G2): convite novo revoga o anterior na mesma transação.
CREATE UNIQUE INDEX convite_um_vivo ON portal.convite (paciente_id) WHERE usado_em IS NULL AND revogado_em IS NULL;

ALTER TABLE portal.convite ENABLE ROW LEVEL SECURITY;
ALTER TABLE portal.convite FORCE ROW LEVEL SECURITY;
-- A recepção emite e vê os convites da unidade; o aceite (sem sessão) passa pelo contexto 'sistema'.
CREATE POLICY convite_equipe ON portal.convite
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'));
GRANT SELECT, INSERT, UPDATE ON portal.convite TO vidacor_app;

ALTER TABLE portal.conta
  ADD COLUMN senha_hash text,
  ADD COLUMN falhas int NOT NULL DEFAULT 0 CHECK (falhas >= 0),
  ADD COLUMN bloqueada_ate timestamptz,
  ADD COLUMN convite_id uuid REFERENCES portal.convite(id),
  ADD CONSTRAINT conta_login_formato CHECK (login ~ '^[a-z0-9._@+-]{4,80}$' OR login ~ '^p-');
GRANT INSERT, UPDATE ON portal.conta TO vidacor_app;

-- Antes do login não há unidade nem usuário no contexto: a busca do convite pelo hash do token é uma
-- função que devolve só o necessário para decidir (sem dado pessoal).
CREATE FUNCTION portal.convite_por_hash(p_hash bytea)
RETURNS TABLE (id uuid, paciente_id uuid, unidade_id uuid, expira_em timestamptz, usado_em timestamptz, revogado_em timestamptz, tentativas int)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public AS $$
  SELECT c.id, c.paciente_id, c.unidade_id, c.expira_em, c.usado_em, c.revogado_em, c.tentativas
    FROM portal.convite c WHERE c.token_hash = p_hash
$$;
GRANT EXECUTE ON FUNCTION portal.convite_por_hash(bytea) TO vidacor_app;

-- Segunda barreira do portal (CLAUDE.md G1): com o contexto do paciente, só o próprio cadastro.
CREATE POLICY paciente_portal ON identidade.paciente FOR SELECT
  USING (id = NULLIF(current_setting('app.paciente_id', true), '')::uuid);
