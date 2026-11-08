-- 0021 · Desfaz a 0018 (BUG-2026-001, correção errada). "Exame pedido" é N3 (docs/18 § 2) e o caixa
-- não vê N3 (§ 4); o teste `caixa_e_financeiro_nao_veem_dado_clinico` provou a quebra. O caixa passa a
-- receber pela rota própria `/v1/caixa/a-receber`, que devolve só paciente (N1), horário (N0) e os
-- procedimentos pela tabela de preços (N2). A 0018 fica no histórico; nada é apagado.
DROP POLICY agendamento_caixa_le ON agenda.agendamento;
DROP POLICY atendimento_caixa_le ON recepcao.atendimento;
