-- 0018 · BUG-2026-001: o caixa vê N0 (operacional) e N1 (cadastral) pela docs/18 § 4 e precisa da
-- agenda do dia com os atendimentos para receber. Só LEITURA (FOR SELECT) e só da própria unidade;
-- nada de triagem, laudo ou dado clínico. As políticas de escrita não mudam.
CREATE POLICY agendamento_caixa_le ON agenda.agendamento FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('caixa'));
CREATE POLICY atendimento_caixa_le ON recepcao.atendimento FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('caixa'));
