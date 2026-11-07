# Registro de riscos clínicos (ISO 14971, versão enxuta)

Aprovado pelo responsável clínico de segurança. Escala: gravidade 1 (desprezível) a 5 (catastrófica); probabilidade 1 (rara) a 5 (frequente). Risco = G × P. **Risco ≥ 12 sem controle não é construído.**

| ID | Perigo | Causa possível | G | P | Risco | Controles | Risco residual | Teste/evidência | Status |
|---|---|---|---|---|---|---|---|---|---|
| RC-001 | Laudo atribuído ao paciente errado | Homônimo; seleção errada na lista | 5 | 3 | 15 | Dois identificadores obrigatórios; alerta de homônimo; conciliação por accession | 5 | Testes de conciliação; usabilidade da tarefa "laudar" | Aberto |
| RC-002 | Imagem de um paciente no estudo de outro | Nome digitado na máquina; worklist não usada | 5 | 3 | 15 | Perfil IHE SWF/PIR; estudo sem conciliação bloqueado para laudo; vínculo manual auditado | 5 | Testes do gateway DICOM | Aberto |
| RC-003 | Alergia ignorada na prescrição ou no contraste | Alerta não exibido ou ignorado | 5 | 2 | 10 | Bloqueio com justificativa do médico; alergia no topo da tela | 5 | Testes de prescrição e checklist da sala | Aberto |
| RC-004 | Resultado delicado visto pelo paciente sem orientação | Publicação automática no portal | 4 | 2 | 8 | Marcação "conversar antes"; publicação só manual nesses casos | 4 | Teste de publicação | Aberto |
| RC-005 | Medida errada no laudo | Erro de digitação; SR mapeado errado | 4 | 3 | 12 | Medidas vindas do aparelho (SR); faixa de referência com aviso; revisão antes de assinar | 4 | Testes de mapeamento SR | Aberto |
| RC-006 | Laudo alterado depois de assinado sem rastro | Edição direta | 5 | 1 | 5 | Registro imutável; nova versão com motivo; auditoria | 5 | Teste de imutabilidade | Aberto |
| RC-007 | Exame sem laudo esquecido | Fila sem prazo | 4 | 3 | 12 | SLA por exame; alerta de atraso; MPPS aberto sinalizado | 4 | Teste do fluxo de laudos | Aberto |
| RC-008 | Dado de saúde exposto a quem não deveria | Permissão errada; vazamento em log | 4 | 2 | 8 | Permissão por ato no servidor; dado de saúde fora de log; auditoria | 4 | Testes de acesso indevido | Aberto |
