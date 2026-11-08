# Pacote de revisão do responsável clínico de segurança

- **Versão do pacote:** v0.1, 08/11/2026
- **Material revisado:** mockups em `design/mockups/` (registrar abaixo o commit ou o SHA-256 dos arquivos) e documentos citados em cada fluxo
- **Revisor:** responsável clínico de segurança (docs/16 C1), médico da VidaCor
- **Regra:** mudança em fluxo clínico só segue com parecer deste pacote (CLAUDE.md). Novo alerta bloqueante exige aprovação explícita (docs/16 R4). Risco com G × P ≥ 12 sem controle não é construído ([registro de riscos](registro-de-riscos-clinicos.md)).

| Campo | Preencher |
|---|---|
| Commit ou SHA-256 dos mockups | |
| Data de recebimento do pacote | |
| Prazo combinado para o parecer | |

## Como usar

1. Abra cada mockup no canvas (modo Play) e percorra o fluxo descrito.
2. Confira os itens de "O que conferir". Marque cada um: **ok**, **ajustar** ou **não se aplica**.
3. Avalie os riscos ligados. Os riscos `RC-0xx` já estão no registro; os marcados **RCP-xx** são **propostos** e só entram no registro com o seu aceite (o ID definitivo é atribuído na inclusão).
4. Dê o parecer de cada fluxo e o parecer geral no fim. "Aprovado com ressalvas" exige a lista de ressalvas, cada uma com prazo; a ressalva vira item rastreável e volta para nova revisão.

## Riscos propostos neste pacote

| ID | Perigo | Causa possível | G | P | Risco | Controle proposto |
|---|---|---|---|---|---|---|
| RCP-01 | Contraste aplicado em paciente com contraindicação | Triagem incompleta ou ignorada | 5 | 2 | 10 | Triagem obrigatória por exame; bloqueio até a conduta médica; TFG calculada pelo servidor |
| RCP-02 | Exame feito no paciente errado | Paciente chamado errado; pulseira de outro | 5 | 2 | 10 | Bipe da pulseira obrigatório; bloqueio com divergência; worklist só após bipe |
| RCP-03 | Categoria de classificação errada no laudo estruturado | Versão da tabela errada; regra mal implementada | 4 | 2 | 8 | Categoria calculada no servidor com versão gravada; casos oficiais testados; texto editável e revisado |
| RCP-04 | Exame externo tomado como laudo da VidaCor ou ligado ao paciente errado | Importação sem conferência; homônimo | 4 | 2 | 8 | Conferência de identidade; marca "externo, somente leitura"; PIR com original guardado |
| RCP-05 | Teleconsulta com paciente não identificado | Identificação só pela câmera | 4 | 2 | 8 | Documento com foto conferido em voz alta; consentimento; três itens antes de abrir a chamada |
| RCP-06 | Discrepância de revisão por pares sem adendo | Fila sem prazo | 4 | 2 | 8 | Prazo de adendo; aviso ao solicitante; acompanhamento pela responsável técnica |
| RCP-07 | Dose alta não percebida | RDSR não lido; NRD desatualizado | 3 | 2 | 6 | Comparação automática com NRD; justificativa obrigatória; revisão do NRD |
| RCP-08 | Troca de paciente na tela clínica sem perceber | Várias abas ou pacientes na sala | 5 | 2 | 10 | Barra do paciente com dois identificadores fixa; troca explícita; rascunho salvo por paciente |
| RCP-09 | Imagem de contingência ligada ao paciente errado | Dados digitados no aparelho na queda | 5 | 2 | 10 | Código de contingência como Patient ID e Accession; conciliação com dois identificadores; fila própria (docs/23) |

---

## Fluxo 1 · Triagem de segurança (`Triagem.dc.html`)

**O que mudou:** questionário por exame com bloqueio (reação prévia, TFG calculada, metformina, gestação, amamentação, jejum, acesso venoso); resultado "precisa de aprovação médica" bloqueia o exame na sala; sugestão de exame sem contraste ao solicitante; termo assinado depois da aprovação; carrinho de emergência conferido. Base: docs/22 E1.

| # | O que conferir | Resultado |
|---|---|---|
| 1.1 | Itens do questionário por tipo de exame e por contraste (iodado, gadolínio, microbolhas) | |
| 1.2 | Fórmula da TFG (CKD-EPI) e limites que levam a "precisa de aprovação médica" | |
| 1.3 | Validade aceita da creatinina (dias) | |
| 1.4 | Conduta médica: quem pode registrar, campos obrigatórios | |
| 1.5 | Protocolo de reação a contraste e checklist do carrinho | |
| 1.6 | Troca para exame sem contraste só com resposta do solicitante | |

**Riscos ligados:** RC-003, RCP-01. **Frases:** FC-TRI-01 a 07 ([docs/27](../27-frases-de-comportamento.md)).

---

## Fluxo 2 · Sala de exame (`Sala-Exame.dc.html`)

**O que mudou:** worklist enviada ao aparelho ("ninguém digita nome na sala"), medidas por DICOM SR direto ao laudo, contraste com lote ligado ao paciente e ao estoque, intercorrência, envio para laudo com prioridade e prazo.

| # | O que conferir | Resultado |
|---|---|---|
| 2.1 | Checklist pré-exame por modalidade | |
| 2.2 | Medidas do SR: quais entram, faixas de referência, aviso fora da faixa | |
| 2.3 | Registro de contraste: campos, quem aplica | |
| 2.4 | Intercorrência: tipos, conduta, quem é chamado | |
| 2.5 | Prioridades e prazos (rotina 24 h, urgente 2 h, internado) | |
| 2.6 | "Exame não realizado": motivos e o que acontece com o paciente | |

**Riscos ligados:** RC-002, RC-005, RC-007, RCP-02.

---

## Fluxo 3 · Laudo (`Laudo.dc.html`)

**O que mudou:** fila com prazo, modelos e frases, sugestão de texto a partir das medidas (marcada para revisão), medidas do aparelho conferidas antes de assinar, mapa de 17 segmentos, comparação com o anterior, pedido de revisão, assinatura ICP-Brasil com confirmação nominal; correção vira versão com motivo.

| # | O que conferir | Resultado |
|---|---|---|
| 3.1 | Confirmação de assinatura com nome e nascimento | |
| 3.2 | Bloqueio de assinatura sem conferência das medidas | |
| 3.3 | Sugestão de texto: marcação, aceitar ou descartar, nada automático | |
| 3.4 | Comparação com o exame anterior: qual é escolhido | |
| 3.5 | Correção: quem pode, motivo, o que o paciente vê no portal | |
| 3.6 | Marcação "conversar antes" e quem libera | |

**Riscos ligados:** RC-001, RC-004, RC-005, RC-006. **Frases:** FC-LAU-01 a 10.

---

## Fluxo 4 · Laudo estruturado (`Laudo-Estruturado.dc.html`)

**O que mudou:** classificações (TI-RADS e demais) calculadas pelo servidor com versão da tabela gravada no laudo; texto gerado das categorias e editável; comparação por nódulo com o exame anterior.

| # | O que conferir | Resultado |
|---|---|---|
| 4.1 | Classificações e versões a usar no MVP (TI-RADS 2017, BI-RADS, obstetrícia) | |
| 4.2 | Pontuação e categoria em casos de referência da tabela oficial | |
| 4.3 | Texto gerado: redação e o que o médico pode mudar | |
| 4.4 | Identificação de nódulo entre exames (N1, N2) | |
| 4.5 | Conduta sugerida pela tabela e responsabilidade do médico | |

**Riscos ligados:** RC-005, RCP-03.

---

## Fluxo 5 · Exame externo (`Exame-Externo.dc.html`)

**O que mudou:** importação por link, arquivo ou DICOMweb; conferência de identidade com o documento; escolha dos estudos; marcação "externo, somente leitura, nunca vira laudo da VidaCor"; correção PIR com original guardado; uso como anterior.

| # | O que conferir | Resultado |
|---|---|---|
| 5.1 | Regras de conferência quando o nome vem abreviado ou diferente | |
| 5.2 | Quem pode importar e quem confirma a identidade | |
| 5.3 | Como o externo aparece na tela de laudo e no portal | |
| 5.4 | O que acontece com o laudo externo que vier junto | |

**Riscos ligados:** RC-001, RC-002, RCP-04.

---

## Fluxo 6 · Etiquetas e rastreio (`Etiquetas.dc.html`)

**O que mudou:** pulseira com código único; bipe na sala confere paciente e exame; pulseira errada bloqueia o exame e abre divergência; reimpressão com motivo invalida a anterior; correção de nome ou nascimento invalida as etiquetas.

| # | O que conferir | Resultado |
|---|---|---|
| 6.1 | Conteúdo da pulseira (nome, nascimento, códigos, aviso de alergia sem detalhe) | |
| 6.2 | Mensagem de bloqueio e encaminhamento à sala certa | |
| 6.3 | Etiqueta da seringa de contraste | |
| 6.4 | Paciente que recusa a pulseira: alternativa | |

**Riscos ligados:** RC-001, RC-002, RCP-02. **Frases:** FC-PUL-01 a 05, FC-ETQ-01 a 05.

---

## Fluxo 7 · Barra do paciente (`Barra-Paciente.dc.html`)

**O que mudou:** barra fixa com nome, nascimento e situação da pulseira em toda tela clínica; troca explícita de paciente com a lista da sala; rascunho salvo ao trocar.

| # | O que conferir | Resultado |
|---|---|---|
| 7.1 | Dados mostrados e destaque de "pulseira não bipada" | |
| 7.2 | Troca de paciente: confirmação e o que muda na tela | |
| 7.3 | Comportamento com dois pacientes de nome parecido | |

**Riscos ligados:** RC-001, RCP-08.

---

## Fluxo 8 · Fluxo completo (`Fluxo-Completo.dc.html`)

**O que mudou:** a cadeia agendar, check-in, sala, laudo e portal em sequência, com o lastro ao lado; guia conferida em 5 itens; check-in com três conferências; bipe com simulação de erro; assinatura com PIN; publicação no portal com aviso.

| # | O que conferir | Resultado |
|---|---|---|
| 8.1 | Ordem das etapas e pontos de bloqueio | |
| 8.2 | Simulação de erro (pulseira de outro paciente) | |
| 8.3 | Publicação no portal logo após a assinatura: quando não deve acontecer | |
| 8.4 | O que o paciente vê no portal | |

**Riscos ligados:** RC-001, RC-002, RC-004, RCP-02.

---

## Fluxo 9 · Telemedicina (`Telemedicina.dc.html`)

**O que mudou:** teleconsulta e telelaudo conforme a Resolução CFM 2.314/2022; identificação pela câmera com documento conferido em voz alta; consentimento; registro de queda de conexão; telelaudo só de estudos conciliados.

| # | O que conferir | Resultado |
|---|---|---|
| 9.1 | Três itens de identificação antes da chamada | |
| 9.2 | Conduta quando a conexão cai (5 min, telefone, remarcação) | |
| 9.3 | Receita e atestado pela Memed na teleconsulta | |
| 9.4 | Telelaudo: prazos e devolução à origem | |

**Riscos ligados:** RC-001, RCP-05.

---

## Fluxo 10 · Revisão por pares (`Revisao-Pares.dc.html`)

**O que mudou:** amostra para segunda leitura (RADPEER), comentário educativo, adendo feito por quem laudou, resultado confidencial que não entra em avaliação de desempenho nem repasse.

| # | O que conferir | Resultado |
|---|---|---|
| 10.1 | Tamanho e critério da amostra (3%) | |
| 10.2 | Classes que exigem adendo e prazo | |
| 10.3 | Aviso ao solicitante em discrepância significativa | |
| 10.4 | Confidencialidade e quem vê o resultado por médico | |

**Riscos ligados:** RC-006, RCP-06.

---

## Fluxo 11 · Gestão de dose (`Gestao-Dose.dc.html`)

**O que mudou:** dose lida do RDSR de cada aparelho, comparada ao NRD do protocolo; justificativa obrigatória acima do NRD; atenção a gestantes e crianças; estimativa clínica, não dosimetria individual.

| # | O que conferir | Resultado |
|---|---|---|
| 11.1 | NRD por protocolo e revisão (última em 03/2026) | |
| 11.2 | Motivos de justificativa | |
| 11.3 | Texto que deixa claro que não é dosimetria individual | |
| 11.4 | Alertas para gestantes e crianças | |

**Riscos ligados:** RCP-07.

---

## Fluxo 12 · Contingência (`Contingencia.dc.html`, [docs/23](../23-modo-contingencia.md))

Incluído porque o procedimento em papel toca identificação, exame com contraste e comunicação de resultado.

| # | O que conferir | Resultado |
|---|---|---|
| 12.1 | Dois identificadores no papel e na sala | |
| 12.2 | Exame com contraste em contingência só com médico presente | |
| 12.3 | Dado digitado no aparelho (código de contingência, nome, nascimento) | |
| 12.4 | Comunicação verbal de resultado urgente (F-COM) | |
| 12.5 | Conciliação das imagens na retomada | |

**Riscos ligados:** RC-001, RC-002, RCP-09. **Frases:** FC-CTG-01 a 11.

---

## Parecer por fluxo

| Fluxo | Parecer (aprovado / aprovado com ressalvas / reprovado) | Ressalvas e prazo | Riscos aceitos para o registro |
|---|---|---|---|
| 1 Triagem | | | |
| 2 Sala de exame | | | |
| 3 Laudo | | | |
| 4 Laudo estruturado | | | |
| 5 Exame externo | | | |
| 6 Etiquetas | | | |
| 7 Barra do paciente | | | |
| 8 Fluxo completo | | | |
| 9 Telemedicina | | | |
| 10 Revisão por pares | | | |
| 11 Gestão de dose | | | |
| 12 Contingência | | | |

## Alertas bloqueantes novos (aprovação explícita, docs/16 R4)

| Alerta | Fluxo | Aprovado (sim / não) |
|---|---|---|
| Triagem "precisa de aprovação médica" bloqueia o exame | 1 | |
| Pulseira de outro paciente bloqueia o exame | 6 | |
| Etiqueta invalidada bloqueia o ato | 6 | |
| Assinatura bloqueada sem conferência das medidas | 3 | |
| Estudo sem conciliação fora da fila de laudo | 2, 12 | |

## Parecer geral

- [ ] Aprovado
- [ ] Aprovado com ressalvas (listadas acima, cada uma com prazo)
- [ ] Reprovado (motivo):

Observações:

**Responsável clínico de segurança:** nome ______________________ CRM/UF ____________

**Assinatura:** ______________________ **Data:** ____/____/________

Este parecer é anexado ao registro de riscos e à revisão de prontidão de cada módulo
clínico; as ressalvas viram itens rastreáveis com dono e prazo.
