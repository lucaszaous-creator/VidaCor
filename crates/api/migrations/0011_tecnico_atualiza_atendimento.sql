-- 0011 · O técnico da sala muda o atendimento para "em exame" no bipe da pulseira (FC-PUL-01);
-- o núcleo (sistema) também atualiza na conciliação. Recepção e gerente continuam.
DROP POLICY atendimento_unidade ON recepcao.atendimento;
CREATE POLICY atendimento_unidade ON recepcao.atendimento
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','tecnico','sistema'));
