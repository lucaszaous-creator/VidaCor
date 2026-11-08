# Matriz de rastreabilidade da Fase 1: frase aprovada para teste

Gerada a partir de `docs/27-frases-de-comportamento.md` e dos testes de integração em `crates/api/tests/`
(rota HTTP real, PostgreSQL real; Orthanc real em `orthanc.rs`). Regenerar a cada mudança de frase ou teste.

| Frase | Cenário | Teste(s) que prova(m) | Situação | Observação |
|---|---|---|---|---|
| FC-AGD-01 | Guia válida é conferida e consumida no agendamento | `agendamento_guia.rs::fc_agd_01_guia_valida_e_consumida_no_agendamento` | Coberta |  |
| FC-AGD-02 | Guia já usada por outro atendimento é recusada | `agendamento_guia.rs::fc_agd_02_guia_ja_usada_e_recusada_com_o_codigo_do_uso` | Coberta |  |
| FC-AGD-03 | Dois agendamentos simultâneos com a mesma guia | `agendamento_guia.rs::fc_agd_03_dois_agendamentos_simultaneos_com_a_mesma_guia` | Coberta |  |
| FC-AGD-04 | Clique duplo no agendar não duplica | `agendamento_guia.rs::fc_agd_04_clique_duplo_nao_duplica` | Coberta |  |
| FC-AGD-05 | Procedimento fora do autorizado é recusado | `agendamento_guia.rs::fc_agd_05_procedimento_fora_do_autorizado_e_recusado` | Coberta |  |
| FC-AGD-06 | Senha vencida na data do exame é recusada | `agendamento_guia.rs::fc_agd_06_senha_vencida_na_data_do_exame_e_recusada` | Coberta |  |
| FC-AGD-07 | Saldo zero de procedimento é recusado | `agendamento_guia.rs::fc_agd_07_saldo_zero_e_recusado` | Coberta |  |
| FC-AGD-08 | Reserva provisória sem guia expira e devolve a vaga | `agendamento_guia.rs::fc_agd_08_reserva_provisoria_vence_devolve_a_vaga_e_avisa` | Coberta em parte | Aviso ao paciente e oferta à lista de espera ficam no outbox; o envio depende do provedor de WhatsApp e do módulo de lista de espera, ainda não construídos. |
| FC-AGD-09 | Guia entregue dentro do prazo confirma a reserva | `agendamento_guia.rs::fc_agd_09_guia_entregue_no_prazo_confirma_a_reserva` | Coberta |  |
| FC-AGD-10 | Perfil sem permissão não agenda | `agendamento_guia.rs::fc_agd_10_perfil_sem_permissao_nao_agenda` | Coberta |  |
| FC-AGD-11 | Dois agendamentos simultâneos no mesmo horário da sala | `agendamento_guia.rs::fc_agd_11_mesmo_horario_da_sala_ao_mesmo_tempo_sugere_proximos_livres` | Coberta |  |
| FC-CHK-01 | Check-in com nome e nascimento conferidos | `recepcao_sala.rs::fc_chk_01_checkin_emite_senha_e_pulseira_e_registra_as_tres_conferencias` | Coberta em parte | Pulseira gerada e registrada; impressão física na impressora do guichê depende do VidaCor Desktop (Tauri), ainda não construído. |
| FC-CHK-02 | Check-in sem os dois identificadores é bloqueado | `recepcao_sala.rs::fc_chk_02_sem_nascimento_ou_sem_documento_bloqueia_e_nada_e_emitido` | Coberta |  |
| FC-CHK-03 | Nascimento diferente do cadastro bloqueia | `checkin.rs::nascimento_divergente_nao_faz_checkin_e_nao_grava_nada` | Coberta |  |
| FC-CHK-04 | Homônimo no mesmo dia gera alerta | `recepcao_sala.rs::fc_chk_04_homonimo_do_dia_aparece_com_os_dois_nascimentos` | Coberta |  |
| FC-CHK-05 | Check-in repetido não gera duas senhas nem duas pulseiras | `checkin.rs::repetir_com_a_mesma_idempotency_key_devolve_a_mesma_resposta; checkin.rs::duas_recepcoes_ao_mesmo_tempo_geram_um_so_atendimento` | Coberta |  |
| FC-CHK-06 | Check-in de agendamento cancelado é recusado | `checkin.rs::agendamento_cancelado_nao_permite_checkin` | Coberta |  |
| FC-CHK-07 | Perfil Caixa não faz check-in | `recepcao_sala.rs::fc_chk_07_perfil_caixa_nao_faz_checkin` | Coberta |  |
| FC-PUL-01 | Pulseira certa libera a worklist | `recepcao_sala.rs::fc_pul_01_pulseira_certa_libera_a_worklist_e_registra_o_bipe` | Coberta |  |
| FC-PUL-02 | Pulseira de outro paciente bloqueia o exame | `recepcao_sala.rs::fc_pul_02_pulseira_de_outra_sala_bloqueia_e_abre_divergencia` | Coberta |  |
| FC-PUL-03 | Pulseira invalidada bloqueia | `recepcao_sala.rs::fc_etq_01_reimpressao_com_motivo_invalida_a_anterior_e_o_bipe_antigo_bloqueia` | Coberta |  |
| FC-PUL-04 | Bipe repetido não envia a worklist duas vezes | `recepcao_sala.rs::fc_pul_04_bipe_repetido_nao_envia_a_worklist_duas_vezes` | Coberta |  |
| FC-PUL-05 | Técnico de outra sala não libera o exame | `recepcao_sala.rs::fc_pul_05_tecnico_de_outra_sala_nao_libera` | Coberta |  |
| FC-TRI-01 | Triagem sem risco libera o exame | `triagem.rs::fc_tri_01_sem_risco_libera_o_exame` | Coberta |  |
| FC-TRI-02 | Reação prévia ou TFG baixa exige aprovação médica | `triagem.rs::fc_tri_02_e_07_tfg_baixa_calculada_pelo_servidor_exige_aprovacao` | Coberta |  |
| FC-TRI-03 | Exame bloqueado não recebe worklist | `triagem.rs::fc_tri_03_exame_bloqueado_nao_recebe_worklist` | Coberta |  |
| FC-TRI-04 | Médico registra a conduta e libera | `triagem.rs::fc_tri_04_medica_com_vinculo_registra_conduta_e_libera` | Coberta |  |
| FC-TRI-05 | Técnico não aprova a própria triagem | `triagem.rs::fc_tri_05_tecnico_nao_aprova_a_propria_triagem` | Coberta |  |
| FC-TRI-06 | Exame com contraste sem triagem não inicia | `triagem.rs::fc_tri_06_exame_com_contraste_sem_triagem_nao_inicia` | Coberta |  |
| FC-TRI-07 | TFG é calculada pelo servidor | `triagem.rs::fc_tri_02_e_07_tfg_baixa_calculada_pelo_servidor_exige_aprovacao` | Coberta |  |
| FC-LAU-01 | Assinar laudo com confirmação nominal e reautenticação | `laudo_portal.rs::fc_lau_01_assinatura_grava_versao_1_com_certificado_carimbo_e_hash` | Coberta em parte | Hash do documento canônico do laudo; o PDF com Typst e a assinatura ICP-Brasil real (serviço JVM) ainda não estão construídos; nos testes, assinador de desenvolvimento. |
| FC-LAU-02 | Laudo em rascunho não vai ao portal | `laudo_portal.rs::fc_lau_02_rascunho_nao_vai_ao_portal` | Coberta |  |
| FC-LAU-03 | Laudo assinado vai ao portal e o paciente é avisado uma vez | `laudo_portal.rs::fc_lau_03_assinado_vai_ao_portal_uma_vez_com_um_aviso` | Coberta em parte | Aviso registrado uma vez; o envio depende do provedor de WhatsApp. |
| FC-LAU-04 | Laudo assinado não pode ser alterado | `laudo_portal.rs::fc_lau_04_laudo_assinado_nao_pode_ser_alterado` | Coberta |  |
| FC-LAU-05 | Correção cria versão 2 com motivo | `laudo_portal.rs::fc_lau_05_retificacao_cria_versao_2_e_o_portal_mostra_a_retificada` | Coberta |  |
| FC-LAU-06 | Correção sem motivo é recusada | `laudo_portal.rs::fc_lau_06_correcao_sem_motivo_e_recusada` | Coberta |  |
| FC-LAU-07 | Duas assinaturas simultâneas geram uma versão vigente | `laudo_portal.rs::fc_lau_07_duas_assinaturas_simultaneas_geram_uma_versao_vigente` | Coberta |  |
| FC-LAU-08 | Marcação "conversar antes" segura a publicação | `laudo_portal.rs::fc_lau_08_conversar_antes_segura_ate_a_liberacao` | Coberta |  |
| FC-LAU-09 | Estudo sem conciliação não entra na fila de laudo | `laudo_portal.rs::fc_lau_09_estudo_sem_conciliacao_nao_entra_na_fila` | Coberta |  |
| FC-LAU-10 | Médico sem atribuição não assina | `laudo_portal.rs::fc_lau_10_medico_sem_atribuicao_nao_assina` | Coberta |  |
| FC-CXA-01 | Total calculado pelo servidor | `caixa_pix.rs::fc_cxa_01_e_02_total_e_do_servidor_com_a_versao_da_tabela` | Coberta |  |
| FC-CXA-02 | Total enviado pela tela é ignorado | `caixa_pix.rs::fc_cxa_01_e_02_total_e_do_servidor_com_a_versao_da_tabela` | Coberta |  |
| FC-CXA-03 | Desconto dentro do limite do perfil | `caixa_pix.rs::fc_cxa_03_desconto_dentro_do_limite_com_motivo` | Coberta |  |
| FC-CXA-04 | Desconto acima do limite sem autorização é recusado | `caixa_pix.rs::fc_cxa_04_desconto_acima_do_limite_sem_autorizacao_e_recusado` | Coberta |  |
| FC-CXA-05 | Gerente autoriza desconto com reautenticação | `caixa_pix.rs::fc_cxa_05_gerente_autoriza_com_reautenticacao_e_o_lastro_guarda_os_dois` | Coberta |  |
| FC-CXA-06 | Recebimento repetido não duplica | `caixa_pix.rs::fc_cxa_06_recebimento_repetido_gera_um_recebimento_e_uma_cobranca_pix` | Coberta |  |
| FC-CXA-07 | Valor zero ou negativo é recusado | `caixa_pix.rs::fc_cxa_07_valor_zero_ou_negativo_e_recusado` | Coberta |  |
| FC-CXA-08 | Recebimento sem caixa aberto é recusado | `caixa_pix.rs::fc_cxa_08_sem_caixa_aberto_e_recusado_e_o_fechamento_mostra_a_diferenca` | Coberta |  |
| FC-CXA-09 | Quem pede o estorno não aprova | `caixa_pix.rs::fc_cxa_09_quem_pede_o_estorno_nao_aprova` | Coberta |  |
| FC-PIX-01 | Pagamento confirmado pelo PSP baixa o recebimento | `caixa_pix.rs::fc_pix_01_e_02_pagamento_baixa_uma_vez_e_o_repetido_para` | Coberta em parte | Pedido da NFS-e fica no outbox uma vez; a emissão depende do provedor de NFS-e. PSP de desenvolvimento nos testes. |
| FC-PIX-02 | Webhook duplicado não gera segundo efeito | `caixa_pix.rs::fc_pix_01_e_02_pagamento_baixa_uma_vez_e_o_repetido_para` | Coberta |  |
| FC-PIX-03 | Assinatura inválida é recusada antes de processar | `caixa_pix.rs::fc_pix_03_assinatura_invalida_e_recusada_antes_de_processar` | Coberta |  |
| FC-PIX-04 | Eventos fora de ordem não regridem a situação | `caixa_pix.rs::fc_pix_04_evento_antigo_depois_do_pago_nao_regride` | Coberta |  |
| FC-PIX-05 | Dois webhooks iguais ao mesmo tempo | `caixa_pix.rs::fc_pix_05_dois_webhooks_iguais_ao_mesmo_tempo` | Coberta |  |
| FC-ETQ-01 | Reimpressão com motivo invalida a anterior | `recepcao_sala.rs::fc_etq_01_reimpressao_com_motivo_invalida_a_anterior_e_o_bipe_antigo_bloqueia` | Coberta |  |
| FC-ETQ-02 | Reimpressão sem motivo é recusada | `recepcao_sala.rs::fc_etq_02_reimpressao_sem_motivo_e_recusada` | Coberta |  |
| FC-ETQ-03 | Impressão duplicada gera uma etiqueta | `recepcao_sala.rs::fc_etq_03_impressao_duplicada_ao_mesmo_tempo_gera_uma_etiqueta` | Coberta |  |
| FC-ETQ-04 | Correção de nascimento invalida as etiquetas | `recepcao_sala.rs::fc_etq_04_correcao_de_nascimento_invalida_as_etiquetas_e_guarda_a_versao` | Coberta |  |
| FC-ETQ-05 | Perfil sem permissão não reimprime | `recepcao_sala.rs::fc_etq_05_perfil_financeiro_nao_reimprime` | Coberta |  |
| FC-ACE-01 | Paciente A não vê laudo de B pelo portal | `laudo_portal.rs::fc_ace_01_e_02_paciente_nao_ve_laudo_de_outro_e_nao_descobre_se_existe` | Coberta |  |
| FC-ACE-02 | Paciente A não descobre se o ID de B existe | `laudo_portal.rs::fc_ace_01_e_02_paciente_nao_ve_laudo_de_outro_e_nao_descobre_se_existe` | Coberta |  |
| FC-ACE-03 | Médico sem vínculo de cuidado não lê laudo | `laudo_portal.rs::fc_ace_03_e_04_sem_vinculo_403_e_acesso_de_emergencia_auditado` | Coberta |  |
| FC-ACE-04 | Acesso de emergência com justificativa é auditado | `laudo_portal.rs::fc_ace_03_e_04_sem_vinculo_403_e_acesso_de_emergencia_auditado` | Coberta |  |
| FC-ACE-05 | RLS barra mesmo se a rota errar | `laudo_portal.rs::fc_ace_05_rls_barra_mesmo_se_a_rota_errar` | Coberta |  |
| FC-ACE-06 | Recepção não lê laudo | `laudo_portal.rs::fc_ace_06_recepcao_nao_le_laudo_e_ve_so_a_situacao` | Coberta |  |
| FC-ACE-07 | TI não lê dado clínico | `laudo_portal.rs::fc_ace_07_ti_nao_le_dado_clinico` | Coberta |  |
| FC-ACE-08 | Link compartilhado expirado não abre | `laudo_portal.rs::fc_ace_08_link_expirado_nao_abre` | Coberta |  |
| FC-CTG-01 | Declaração de contingência | `contingencia.rs::fc_ctg_01_e_02_declaracao_unica_com_a_hora_original` | Coberta |  |
| FC-CTG-02 | Declaração feita offline sobe uma vez | `contingencia.rs::fc_ctg_01_e_02_declaracao_unica_com_a_hora_original` | Coberta |  |
| FC-CTG-03 | Ficha lançada leva a marca de contingência | `contingencia.rs::fc_ctg_03_ficha_lancada_leva_a_marca_de_contingencia` | Coberta em parte | Hash do PDF digitalizado gravado; o armazenamento do arquivo ainda não está construído. |
| FC-CTG-04 | Mesma ficha lançada duas vezes gera um registro | `contingencia.rs::fc_ctg_04_mesma_ficha_em_duas_estacoes_gera_um_registro` | Coberta |  |
| FC-CTG-05 | Ficha com número fora da faixa é recusada | `contingencia.rs::fc_ctg_05_ficha_fora_da_faixa_e_recusada` | Coberta |  |
| FC-CTG-06 | Nascimento diferente no papel abre divergência | `contingencia.rs::fc_ctg_06_nascimento_diferente_no_papel_abre_divergencia_sem_mexer_no_cadastro` | Coberta |  |
| FC-CTG-07 | Valor do papel diferente do recalculado abre divergência | `contingencia.rs::fc_ctg_07_valor_do_papel_diferente_grava_o_recalculado_e_abre_divergencia` | Coberta |  |
| FC-CTG-08 | Estudo de contingência não vai direto ao laudo | `contingencia.rs::fc_ctg_08_estudo_de_contingencia_vai_para_fila_propria_e_so_entra_no_laudo_conciliado` | Coberta |  |
| FC-CTG-09 | Encerramento bloqueado com pendência | `contingencia.rs::fc_ctg_09_encerramento_bloqueado_com_pendencia_e_liberado_depois` | Coberta |  |
| FC-CTG-10 | Perfil sem permissão não declara nem encerra | `contingencia.rs::fc_ctg_10_perfil_caixa_nao_declara_nem_encerra` | Coberta |  |
| FC-CTG-11 | Kit das 06:00 é gerado com faixa reservada | `contingencia.rs::fc_ctg_11_kit_do_dia_reserva_faixas_sem_sobrepor_e_grava_hash` | Coberta em parte | Kit gerado como texto com hash; o PDF com Typst ainda não está construído. |

Frases: 78, todas com teste de integração. Cobertas em parte (dependem de peça ainda não construída): 7, listadas na coluna Observação.
