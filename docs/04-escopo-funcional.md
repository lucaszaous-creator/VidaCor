# 04 · Escopo funcional

Cada módulo, com as funções que ele tem e a tela do mockup onde aparecem. A
coluna **Origem** diz se a função vem da Medware (MW), da Medilab (ML), do
pedido do cliente (Cliente) ou se é proposta nossa (Nova).

## Clínica

| Módulo / tela | Funções | Origem |
|---|---|---|
| **Início** (`Main`) | Números do dia, produção da semana, próximos pacientes, ocupação por sala, pendências que travam faturamento, caixa do dia, estacionamento, NPS | MW Metrics + Nova |
| **Agenda** (`Agenda`) | Dia por sala e equipamento, horários livres e encaixe, buscador de próximos horários, lista de espera, bloqueios e feriados, agendamento online para aprovar, orçamento do particular (WhatsApp), elegibilidade e autorização, preparo automático, confirmações | MW + ML + Cliente |
| **Recepção e senhas** (`Recepcao`) | Senhas por tipo (normal, preferencial, resultado), guichês, chamar/rechamar, linha do dia com horários livres, check-in em cartões (identidade, elegibilidade, guia, pedido por foto, termo no tablet, pagamento), etiquetas, placa validada, envio para a sala, painel da TV | MW + Cliente |
| **Caixa do particular** (`Caixa`) | Abertura com fundo de troco, suprimento, sangria, fechamento com conferência (esperado × contado) e assinaturas, recebimento com itens, desconto com autorização do gerente, pagamento dividido (PIX com QR, cartão parcelado, dinheiro com troco), NFS-e automática, recibo, comprovante por WhatsApp, estorno com motivo, orçamentos abertos, tabela do particular | MW + Cliente |
| **Etiquetas e rastreio** (`Etiquetas`) | Pulseira, exame, seringa, MAPA, envelope e CD; código único; bipe na sala com bloqueio de divergência; reimpressão com motivo que invalida a anterior; lastro de eventos; impressoras por local | MW (etiquetas) + Nova |
| **Paciente 360º** (`Paciente`) | Cadastro, alertas e alergias, **mapa anatômico realista** com exames por região, linha do tempo com imagens, evolução de medidas, documentos, financeiro, consentimentos LGPD, auditoria, acesso ao portal | MW + Cliente + Nova |
| **TV de chamada** (`Telao`) | Senha e sala atuais, últimas chamadas, nome abreviado (privacidade), mensagens | MW |

## Atendimento, exames e imagens

| Módulo / tela | Funções | Origem |
|---|---|---|
| **Atendimento médico** (`Atendimento`) | Meu dia, anamnese, evolução SOAP, exame físico, CID-10, prescrição (Memed, alerta de interação e alergia), pedidos de exame (TUSS, guia SADT automática), atestado, declaração, relatório, encaminhamento com ICP-Brasil, sinais vitais, anatomia, teleconsulta, finalizar (bloqueia; correção vira versão) | MW + Cliente |
| **Sala de exame** (`Sala-Exame`) | Worklist DICOM enviada ao aparelho, status dos aparelhos, captura de imagem e cine, medidas por DICOM SR, checklist pré-exame, contraste (lote, volume) ligado ao estoque, pausa e intercorrência, envio para laudo com prioridade | MW comDICOM + ML |
| **Aparelhos e integração DICOM** (`Aparelhos`) | Caminho do exame (check-in → worklist → MPPS → imagens e SR → conferência → laudo), sala ao vivo com imagens chegando e medidas, revisão de vínculo paciente × imagem (auditada), conversor para DICOM (PDF do ergométrico e do MAPA, captura HDMI), status de todos os aparelhos, registro de comunicação, configuração do servidor (AE Title, porta, TLS) | MW comDICOM + ML MediPrint + Nova |
| **Laudar** (`Laudo`) | Lista com prioridade/SLA, modelos, banco de frases, ditado, rascunho de IA para revisar, medidas automáticas com referência, anatomia cardio, mapa de 17 segmentos do VE, comparação com o anterior, revisão por par, assinatura ICP-Brasil | MW Laudos + ML Mediworks |
| **Visualizador** (`Visualizador`) | Séries, janela/nível, zoom, medidas, ângulo, ROI, anotações, MPR/3D, cine, comparação lado a lado, metadados DICOM, imagens-chave, gravar CD/exportar | ML PACS |
| **Fluxo de laudos** (`Fluxo-Laudos`) | Quadro por etapa, prazos (SLA), filtros, produtividade por médico, entrega (portal, WhatsApp), impressão sob demanda e CD/DVD, telerradiologia, laudo retificado com versão | ML Mediflow |

## Gestão

| Módulo / tela | Funções | Origem |
|---|---|---|
| **Faturamento TISS** (`Faturamento`) | Guias com semáforo e prazo, conferência antes do lote, lote XML TISS 4.01 validado no schema ANS, transmissão, demonstrativo de retorno, glosas e recurso, tabelas TUSS/CBHPM | MW (nossa base Clinica já tem) |
| **Financeiro** (`Financeiro`) | Fluxo de caixa, contas a pagar/receber, conciliação, DRE, centro de custo, previsão, NFS-e, repasse médico, integração com o caixa | MW |
| **Estoque** (`Estoque`) | Itens, lote e validade (FEFO), mínimo, baixa automática por exame, compras e fornecedores, curva ABC, inventário, rastreio de lote de contraste por paciente | MW |
| **WhatsApp e NPS** (`Comunicacao`) | Caixa de conversas da equipe, respostas rápidas, sugestão de resposta, automações (confirmação, preparo, laudo pronto, NPS, aniversário), campanhas, robô de agendamento com passagem para um atendente, opt-out LGPD, telemedicina | MW Paciente Conectado / MultiChat / Telemedicina + ML Autonotify |
| **Indicadores** (`Indicadores`) | Receita × meta, produção por médico e exame, tempo de laudo, faltas, ocupação por sala, glosas, ticket médio, NPS, origem dos pacientes, filtros e exportação | MW Metrics |
| **Configurações** (`Configuracoes`) | Usuários, perfis e permissões por ato, 2FA, auditoria, aparelhos DICOM (AE Title, IP, porta, worklist), modelos de laudo, exames e preparos, convênios e regras, preços do particular, impressoras e etiquetas, horários por sala e feriados, integrações (Memed, NFS-e, WhatsApp, PIX, estacionamento), backup, LGPD, marca | MW + Nova |
| **Estacionamento** | Entrada, saída e permanência por placa, validação pelo exame, cancela (ver [02](02-estacionamento-leitura-de-placa.md)) | Cliente |

## Paciente e médico

| Tela | Funções | Origem |
|---|---|---|
| **Portal web** (`Portal-Web`) | Próximo exame e preparo, confirmar e remarcar, agendamento online, histórico, laudos, imagens, documentos, recibos e notas, anatomia, compartilhar com o médico (link que expira), consentimentos, baixar dados, 2FA, acessibilidade | MW Meus Exames + ML MediWeb + Cliente |
| **App do paciente** (`App-Paciente`) | Início, próximo exame, resultados, documentos | MW |
| **Meus Exames Bebê** (`App-Bebe`) | Semanas de gestação, imagens e vídeos, compartilhar com a família, medidas, resultado sensível só após conversa médica | MW |
| **App do médico** (`App-Medico`) | Agenda do dia, laudos para assinar, imagens, assinatura no app, produção | MW App Agenda |

## Documentos impressos

Laudo A4 com timbrado, assinatura ICP-Brasil e QR de validação · Comprovante e
preparo com QR para check-in · Atestado de comparecimento · Recibo · Etiquetas
de identificação · Etiqueta de CD/DVD.

## Fora do escopo por enquanto

- **Totem de autoatendimento**: o cliente disse que é desnecessário.
- Maquininha de cartão integrada: depois do PIX.
