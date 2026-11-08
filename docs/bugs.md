# Bugs conhecidos

Bugs encontrados por testes que falham por **comportamento errado do sistema** (não do teste).
Regra (G4): o teste continua falhando até o bug ser corrigido; ele não é apagado, pulado nem afrouxado.

| ID | Descrição | Teste que reproduz | Encontrado em | Status |
|---|---|---|---|---|
| BUG-2026-001 | O perfil Caixa recebia 403 na agenda do dia e o RLS escondia agendamentos e atendimentos, então a tela do caixa não listava os pacientes com check-in contra o núcleo real (nos testes da tela o núcleo simulado respondia). docs/18 § 4 dá N0 e N1 ao caixa | `crates/api/tests/pendencias_caixa.rs::caixa_recebe_pela_lista_a_receber_sem_ler_a_agenda` | 08/11/2026, ao montar o menu de contexto | Corrigido em duas etapas: a primeira correção (migration 0018, caixa lendo a agenda) quebrou a regra N3 do docs/18 e foi pega pelo teste `caixa_e_financeiro_nao_veem_dado_clinico`; desfeita na migration 0021. Correção final: rota `/v1/caixa/a-receber` (paciente N1, horário N0, procedimentos pela tabela de preços N2), teste `caixa_recebe_pela_lista_a_receber_sem_ler_a_agenda` |
