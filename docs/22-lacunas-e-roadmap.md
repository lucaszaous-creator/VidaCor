# 22 · Lacunas e roteiro

> **Status:** aprovado pelo dono do produto em 07/11/2026 (exceto o item marcado
> como descartado). Revisão do projeto contra o que clínicas grandes de
> diagnóstico por imagem usam.

## Decisões
| Item | Decisão |
|---|---|
| Comunicação obrigatória de achado crítico ao solicitante | **Descartado** pelo dono do produto (07/11/2026). Pode voltar à pauta se a clínica pedir |
| Todos os demais itens abaixo | **Aprovados** para o roteiro |

## 🔴 Essencial (antes de construir / no MVP)

| # | Item | O que é | Mockup / documento |
|---|---|---|---|
| E1 | **Triagem de segurança** | Questionário por exame com bloqueio: reação prévia a contraste, função renal (TFG calculada), metformina, gestação, amamentação, jejum, acesso venoso; itens de ressonância quando houver. Resultado: liberado, liberado com ressalvas ou **precisa de aprovação médica** (exame bloqueado na sala até o médico registrar a conduta). Protocolo de reação e checklist do carrinho de emergência | `Triagem.dc.html` |
| E2 | **Laudo estruturado e classificações** | TI-RADS, BI-RADS, O-RADS, LI-RADS, Lung-RADS, Bosniak, eco (ASE) e obstetrícia (biometria com percentis). Categoria e conduta calculadas pela versão vigente da tabela (guardada no lastro); texto gerado e editável; comparação com o anterior | `Laudo-Estruturado.dc.html` |
| E3 | **Exame externo** | Importar CD/pendrive/link de outra instituição: conferência de identidade, escolha dos estudos, marcação como externo (somente leitura), correção de identificação (IHE PIR) com original guardado, uso como exame anterior | `Exame-Externo.dc.html` |
| E4 | **Pedido de autorização pelo sistema** | Montar a solicitação a partir do agendamento (carteirinha, TUSS, CID, pedido anexado) e enviar por **webservice TISS** às operadoras que aceitam; para as demais, montar o pedido, enviar pelo portal e registrar a resposta. Fila com situação (em análise, autorizada, pendente de documento, negada com motivo), prazo e lastro; resposta preenche a guia sem digitação | `Autorizacao.dc.html` |
| E5 | **Auditoria antes do faturamento** | Regras por operadora, versionadas: senha vencida, procedimento diferente do autorizado, laudo não assinado, CID ausente, assinatura do beneficiário, valor fora da tabela, prazo de envio, carteirinha, elegibilidade. Guia com erro bloqueante não entra no lote; glosa recebida sugere regra nova | `Auditoria-Faturamento.dc.html` |
| E6 | **Modo contingência** | Procedimento oficial quando o sistema ou a rede caem: agenda do dia e fichas exportadas em PDF toda manhã, formulários em papel numerados, retomada com lançamento posterior e conciliação (lastro marca "registrado em contingência") | Documento a fazer |
| E7 | **Plano de migração** (Medware/Medilab) | Inventário de dados, exportação, mapeamento, carga de teste, conferência por amostragem, período em paralelo, critério de virada e de volta atrás; imagens pelo PACS (DICOM), cadastros e laudos por exportação dos fornecedores | Documento a fazer |
| E8 | **Plano do projeto** | Escopo do MVP, fases e datas, equipe, orçamento, critérios de aceite por módulo, contrato com a clínica (SLA, suporte, propriedade e devolução dos dados) | Documento a fazer |

## 🟠 Importante (logo após o MVP)

| # | Item | O que é |
|---|---|---|
| I1 | **Qualidade e acreditação** | Preparação para **PADI** (CBR) e/ou ONA: POPs versionados com treinamento obrigatório, não conformidades, ações corretivas e preventivas, indicadores exigidos, **notificação de eventos adversos (Notivisa)** |
| I2 | **Revisão por pares dos laudos** | Amostragem para segunda leitura, registro de discrepância, taxa de adendos por médico, de forma educativa e confidencial |
| I3 | **Corpo clínico** | CRM, RQE, validades, certificado digital; credenciamento por exame (quem pode laudar o quê); escala; distribuição automática de laudos por especialidade e prazo |
| I4 | **Gestão de dose** | Dose por exame comparada aos níveis de referência, alerta de dose alta, histórico de dose do paciente (os dados já são rastreados no docs/21) |
| I5 | **Portal do médico solicitante** | Médicos de fora pedem exames e acompanham os resultados dos seus pacientes; relacionamento com quem encaminha |
| I6 | **Convênios a fundo** | Contratos, tabelas negociadas, reajustes e prazos por operadora (alimentam E4 e E5) |
| I7 | **Check-up e pacotes** | Vários exames no mesmo dia com roteiro (ordem por preparo e jejum), pacotes para empresas e saúde ocupacional |
| I8 | **Agenda com vários recursos** | Sala + médico + técnico + aparelho + sedação/anestesia ao mesmo tempo; regras de pediatria e gestantes |
| I9 | **Ouvidoria / SAC** | Reclamações e elogios com prazo, responsável, resposta e indicador |

## 🟢 Futuro (diferenciais)

| # | Item |
|---|---|
| F1 | Previsão de faltas, encaixe inteligente e previsão de demanda |
| F2 | IA de apoio à detecção nas imagens e reconstruções 3D (avaliação Anvisa) |
| F3 | Interoperabilidade com hospitais (HL7 v2, FHIR, RNDS), compartilhamento de imagens entre instituições, exportação contábil |
| F4 | Várias unidades e novas modalidades (ressonância, mamografia, densitometria, medicina nuclear com rastreio de material radioativo) |
| F5 | Documentação LGPD completa: RIPD, registro das operações, contrato de operador com a clínica, termos e política do portal |
| F6 | Telemedicina e telelaudo conforme a Resolução CFM 2.314/2022 |

## Regras novas que decorrem destes itens
- **Triagem (E1):** exame com triagem "precisa de aprovação médica" fica **bloqueado** na sala; a aprovação grava médico, conduta e horário (docs/16 R1, docs/21).
- **Classificações (E2):** a versão da tabela usada (ex.: ACR TI-RADS) é gravada no laudo; mudança de versão não altera laudos antigos.
- **Exame externo (E3):** nunca vira laudo da VidaCor; identificação corrigida guarda o original; importação com hash e lastro.
- **Autorização (E4):** a solicitação usa os mesmos dados do agendamento (sem redigitação); resposta da operadora é gravada como recebida; chave de idempotência por solicitação (G3).
- **Auditoria (E5):** regras versionadas por operadora com vigência; guia com erro bloqueante não entra em lote; toda glosa vira candidata a regra.
