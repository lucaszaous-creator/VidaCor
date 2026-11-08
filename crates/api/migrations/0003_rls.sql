-- 0003 · RLS forçado nas tabelas com dado de paciente (docs/18 § 7, REQ-LAS-002).
-- Contexto declarado pelo núcleo a cada transação com SET LOCAL (set_config(..., true)).
-- Sem contexto, nada aparece (negar por padrão). Proibido USING (true).

CREATE FUNCTION comum.ctx_unidade() RETURNS uuid LANGUAGE sql STABLE AS
$$ SELECT NULLIF(current_setting('app.unidade_id', true), '')::uuid $$;

CREATE FUNCTION comum.ctx_tem_perfil(VARIADIC p text[]) RETURNS boolean LANGUAGE sql STABLE AS
$$ SELECT coalesce(string_to_array(NULLIF(current_setting('app.perfis', true), ''), ',') && p, false) $$;

-- Quem lida com o atendimento do dia, e o próprio núcleo ('sistema': outbox e conciliação).
-- TI e financeiro não veem dado de paciente (docs/18 § 4). 'sistema' nunca vem do login: é
-- declarado só pelos trabalhos internos do núcleo.
CREATE FUNCTION comum.ctx_atendimento() RETURNS boolean LANGUAGE sql STABLE AS
$$ SELECT comum.ctx_tem_perfil('recepcao','tecnico','medico','gerente','sistema') $$;

ALTER TABLE identidade.paciente ENABLE ROW LEVEL SECURITY;
ALTER TABLE identidade.paciente FORCE ROW LEVEL SECURITY;
CREATE POLICY paciente_unidade ON identidade.paciente
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente'));

ALTER TABLE agenda.agendamento ENABLE ROW LEVEL SECURITY;
ALTER TABLE agenda.agendamento FORCE ROW LEVEL SECURITY;
CREATE POLICY agendamento_unidade ON agenda.agendamento
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente'));

ALTER TABLE recepcao.atendimento ENABLE ROW LEVEL SECURITY;
ALTER TABLE recepcao.atendimento FORCE ROW LEVEL SECURITY;
CREATE POLICY atendimento_unidade ON recepcao.atendimento
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente'));

ALTER TABLE exames.estudo ENABLE ROW LEVEL SECURITY;
ALTER TABLE exames.estudo FORCE ROW LEVEL SECURITY;
CREATE POLICY estudo_unidade ON exames.estudo
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema','recepcao','tecnico'));

-- Funções usadas pelas políticas: a aplicação só executa.
GRANT EXECUTE ON FUNCTION comum.ctx_unidade(), comum.ctx_tem_perfil(text[]), comum.ctx_atendimento(),
  comum.proximo_numero(text, int) TO vidacor_app;

-- Lastro também respeita perfis (docs/21 § 5 item 8): só a unidade do contexto; o DPO/auditor lê
-- os metadados. Os dados do evento guardam ids e códigos, nunca nome ou conteúdo clínico.
ALTER TABLE lastro.evento ENABLE ROW LEVEL SECURITY;
ALTER TABLE lastro.evento FORCE ROW LEVEL SECURITY;
CREATE POLICY evento_unidade ON lastro.evento
  USING (unidade_id = comum.ctx_unidade() AND (comum.ctx_atendimento() OR comum.ctx_tem_perfil('auditor')))
  WITH CHECK (unidade_id = comum.ctx_unidade());
