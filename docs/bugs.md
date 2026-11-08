# Bugs conhecidos

Bugs encontrados por testes que falham por **comportamento errado do sistema** (não do teste).
Regra (G4): o teste continua falhando até o bug ser corrigido; ele não é apagado, pulado nem afrouxado.

| ID | Descrição | Teste que reproduz | Encontrado em | Status |
|---|---|---|---|---|
| BUG-2026-001 | O perfil Caixa recebia 403 na agenda do dia e o RLS escondia agendamentos e atendimentos, então a tela do caixa não listava os pacientes com check-in contra o núcleo real (nos testes da tela o núcleo simulado respondia). docs/18 § 4 dá N0 e N1 ao caixa | `crates/api/tests/pendencias_caixa.rs::caixa_ve_a_agenda_do_dia_com_os_atendimentos_para_receber` | 08/11/2026, ao montar o menu de contexto | Corrigido: migration 0018 (só leitura, mesma unidade) e `Ato::AgendaVer` para o caixa |
