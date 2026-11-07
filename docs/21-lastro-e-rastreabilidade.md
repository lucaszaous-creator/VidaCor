# 21 · Lastro e rastreabilidade

> **Status:** regra obrigatória (v0.1, nov/2026). Decisão no [ADR-0018](adr/0018-lastro-e-rastreabilidade.md).
> Mockups: `Agendamento-Guia.dc.html` (guia no agendamento) e `Lastro.dc.html`
> (explorador da cadeia do atendimento). Etiquetas em [docs/20](20-etiquetas-e-rastreio.md).

**Princípio:** no VidaCor **nada existe solto**. Todo registro sabe de onde veio,
quem o criou, por qual canal e equipamento, a que outros registros está ligado e
o que aconteceu com ele depois. A partir de **qualquer código** (accession, guia,
etiqueta, StudyInstanceUID, NFS-e, PIX, lote, placa) chega-se à **cadeia
completa** do atendimento.

---

## 1. Metadados obrigatórios em todo registro

| Metadado | Conteúdo |
|---|---|
| `id` | UUIDv7 (ordenável no tempo) + código legível do domínio (ex.: `ATD-2026-118790`) |
| `criado_em` / `criado_por` | Data e hora (UTC) e usuário **ou** sistema/aparelho/integração que criou |
| `origem` | Canal: balcão, WhatsApp (robô ou humano), portal, app, aparelho DICOM, integração, importação, migração |
| `estacao` | Máquina ou equipamento (guichê 2, VIVID_S1, impressora G2) e IP interno |
| `unidade` | Unidade da clínica |
| `versao` | Número da versão (registros clínicos e regras são versionados, nunca sobrescritos) |
| `correlacao` | `trace_id` da requisição e `id` do atendimento a que pertence |
| `ligacoes` | Referências tipadas a outros registros (seção 3) |
| `hash` | SHA-256 de todo documento, imagem e arquivo anexado |
| `regra_aplicada` | Versão da regra de negócio usada (tabela de preço, regra de convênio, modelo de laudo, modelo de mensagem) |
| `motivo` | Obrigatório em correção, cancelamento, estorno, reimpressão, desbloqueio, acesso de emergência |

Implementação: colunas padrão em todas as tabelas + **tabela de eventos
imutável** (append-only, com hash encadeado) por domínio + tabela de **ligações**
entre registros. Nada é apagado; o que muda gera evento.

## 2. A cadeia do atendimento (o "fio" principal)

```
Agendamento ─┬─ Guia/autorização ── Elegibilidade ── Pedido médico (doc + hash)
             ├─ Confirmação (mensagem, modelo, status)
             └─ Atendimento (check-in) ─┬─ Pulseira/etiquetas (ETQ)
                                        ├─ Termos assinados (versão + hash)
                                        ├─ Pagamento particular (recibo, PIX, NFS-e)
                                        ├─ Estacionamento (placa)
                                        └─ Pedido de exame (ACC) ─┬─ Worklist enviada (aparelho)
                                                                  ├─ MPPS (início/fim, operador)
                                                                  ├─ Estudo DICOM (StudyUID, séries, SOPs, hash)
                                                                  ├─ Medidas (DICOM SR) · dose (RDSR)
                                                                  ├─ Insumos (contraste: lote; transdutor: desinfecção)
                                                                  └─ Laudo v1..vn ─┬─ Assinatura ICP (certificado, carimbo do tempo)
                                                                                   ├─ Entrega (portal, impresso, CD, retirada)
                                                                                   └─ Faturamento ─┬─ Guia TISS (XML)
                                                                                                   ├─ Lote (protocolo ANS)
                                                                                                   ├─ Demonstrativo / glosa / recurso
                                                                                                   ├─ Recebimento (conciliação)
                                                                                                   └─ Repasse ao médico · NFS-e
```

## 3. Domínios de lastro (tudo que clínicas grandes rastreiam)

### 3.1 Paciente e cadastro
- Histórico de **todas** as alterações de cadastro (antes/depois, quem, motivo); documento de identidade conferido (tipo, número parcial, quem conferiu).
- **Unificação de duplicados** com registro dos dois cadastros de origem e de quem aprovou (duas pessoas).
- Responsável legal, acompanhante e terceiros autorizados (procuração para retirar resultado).
- Consentimentos LGPD por finalidade com versão do texto aceito.

### 3.2 Agenda
- Quem agendou, por qual canal, quando; cada **remarcação, troca de médico/sala, encaixe, bloqueio e cancelamento** com motivo.
- Lista de espera: quem ofereceu a vaga, quem aceitou, em que ordem.
- Confirmações enviadas (modelo e versão), entregues, lidas e respondidas; no-show registrado.

### 3.3 Convênio, guia e autorização
- **Regra da clínica:** paciente de convênio **só é atendido com guia autorizada**. O agendamento de convênio **exige a seção da guia** (mockup `Agendamento-Guia`):
  - beneficiário: operadora, plano, carteirinha e validade;
  - guia: nº na operadora, **senha de autorização**, data da autorização, **validade da senha**, caráter, tipo de guia TISS;
  - **procedimentos autorizados** (TUSS e quantidade) e **saldo**;
  - solicitante: nome, CRM/UF, CBO, data do pedido, CID-10 quando exigido, indicação clínica;
  - anexos: pedido médico e autorização (com hash).
- **Conferências automáticas:** carteirinha confere com o paciente; elegível; procedimento agendado ⊂ autorizado; quantidade com saldo; senha válida na data do exame; **guia não usada por outro atendimento** (índice único operadora + nº da guia + procedimento, consumo atômico — G2); prazo de validade do pedido conforme a operadora.
- Sem guia: **reserva provisória** (padrão 48 h, configurável) com pedido automático ao paciente; vencido o prazo, a vaga volta para a lista de espera.
- Guia vencendo antes do exame gera alerta para remarcar ou pedir prorrogação.
- Protocolos de elegibilidade e autorização guardados (número, data, resposta).

### 3.4 Recepção e check-in
- Cada etapa do check-in com quem, quando e evidência (foto do documento, assinatura do termo, protocolo de elegibilidade).
- Senhas: emitida, chamada (guichê, quem), tempo de espera.
- Etiquetas (docs/20): impressão, colocação, bipes, reimpressões, invalidações.

### 3.5 Exame e aparelhos (IHE SWF/PIR)
- Accession → worklist enviada (aparelho, hora) → **MPPS** (início, fim, operador, descontinuado e por quê) → estudo (StudyInstanceUID, séries, SOP Instance UIDs, número de objetos, **hash de cada objeto**).
- Conciliação paciente × estudo (automática ou manual, quem, motivo); correções de identificação nas imagens (PIR) com antes/depois.
- **Dose de radiação** por exame em RX e TC (DLP, CTDIvol, DAP) a partir do relatório estruturado de dose do aparelho (RDSR), ligada ao paciente: exigência de **proteção radiológica (Anvisa RDC 611/2022)**, também útil para histórico de dose do paciente.
- **Insumos por exame:** contraste (produto, lote, validade, volume, via, quem administrou, reação adversa se houver); gel, materiais.
- **Transdutores de ultrassom:** desinfecção (nível, produto, lote, ciclo, quem fez) ligada ao **exame anterior e ao seguinte**, em especial transdutores endocavitários.
- Intercorrências na sala (reação ao contraste, mal-estar), com conduta e responsável.

### 3.6 Equipamentos
- Cadastro com número de série, AE Title, IP, versão de software, **DICOM Conformance Statement**.
- **Manutenção preventiva e corretiva**, **controle de qualidade e testes de constância** (exigidos em radiodiagnóstico), calibração, contratos e laudos técnicos.
- Disponibilidade (paradas, motivo, exames afetados e remarcados).
- Dosimetria dos profissionais expostos e registros da proteção radiológica.

### 3.7 Laudo e documento clínico
- Versões do laudo (rascunho, assinado, retificado), **o que mudou e por quê**; modelo e frases usados; medidas vindas do aparelho × digitadas.
- Rascunho de IA: modelo e versão, o que foi aceito, editado ou descartado pelo médico.
- Assinatura: certificado (titular, emissor, série), **carimbo do tempo**, hash do PDF.
- Segunda leitura, revisão por par, telerradiologia (quem, quando, prazo).
- Marcação "conversar antes de liberar" e quem liberou depois.

### 3.8 Entrega do resultado
- Publicação no portal (quando, avisos enviados, quando o paciente abriu).
- Impresso e CD: quem imprimiu e gravou, quantas vias, **quem retirou** (paciente, responsável, terceiro com procuração), documento e assinatura de retirada.
- Compartilhamento com médico externo: link, validade, acessos, revogação.

### 3.9 Faturamento (TISS)
- Guia TISS gerada do atendimento (versão do XML, regra do convênio aplicada).
- **Conferência antes do lote** (quem conferiu), lote (número, hash do XML), envio (protocolo ANS), **demonstrativo de retorno**, glosa (código, motivo), **recurso** (texto, prazo, resultado).
- Ligação valor faturado × valor recebido × conciliação bancária.

### 3.10 Caixa e financeiro
- Abertura, recebimentos, sangria, suprimento, fechamento (esperado × contado, diferença, quem conferiu).
- Cada recebimento: itens, **tabela de preço vigente na data (versão)**, desconto (quem autorizou, motivo), forma, **txid/endToEndId do PIX**, NSU do cartão, **NFS-e** (número, código de verificação), estorno (quem pediu, quem aprovou).
- Repasse médico: regra aplicada (versão), base de cálculo, atendimentos que compõem, pagamento.
- Contas a pagar: nota do fornecedor, aprovação, pagamento, conciliação.

### 3.11 Estoque
- Lote, validade, fornecedor, nota de entrada; **consumo por exame e por paciente**; ajustes com motivo; inventários.
- **Recall:** dado um lote, lista de pacientes que receberam.
- Condições de armazenamento quando aplicável (temperatura do refrigerador de medicamentos).

### 3.12 Comunicação
- Toda mensagem: canal, **modelo e versão**, destinatário (paciente interno), status (enviada, entregue, lida, respondida, falhou), opt-out.
- Conversas humanas no WhatsApp: quem atendeu, transferências, tempo de resposta.

### 3.13 Acesso, segurança e LGPD
- **Toda leitura** de dado clínico (quem, quando, de onde, qual registro) e todo acesso de emergência com justificativa e revisão.
- Login, 2FA, sessões, mudanças de permissão (quem pediu, quem aprovou).
- Exportações (o quê, quem, escopo, hash do arquivo).
- Pedidos de titulares (acesso, correção, exclusão) e resposta.

### 3.14 Estacionamento
- Placa ↔ paciente/acompanhante ↔ atendimento; entrada, saída, cortesia aplicada, pagamento.

### 3.15 Integrações
- Cada chamada externa e webhook: provedor, chave de idempotência, ID do evento, payload resumido (sem dado sensível desnecessário), resposta, protocolo, tentativas e falhas (G3).

### 3.16 Sistema e configuração
- Mudanças de configuração (tabelas de preço, regras de convênio, modelos de laudo, modelos de mensagem, preparos, horários) **versionadas** com vigência, quem mudou e quem aprovou.
- Versões de software instaladas por máquina e data (docs/13); migrações executadas.
- Backups e testes de restauração (quando, resultado).
- Incidentes e análises pós-incidente ligados aos registros afetados.

## 4. Como o lastro é usado

| Uso | Exemplo |
|---|---|
| **Explorador de lastro** | Digitar `ACC 2026-118790` e ver a cadeia inteira, com metadados de cada nó |
| **Linha do tempo** em cada ficha | Paciente, exame, guia, laudo, aparelho, lote: aba "Lastro" |
| **Dossiê do atendimento** | PDF assinado com toda a cadeia, para auditoria do convênio, defesa de glosa, processo ético ou judicial |
| **Recall e investigação** | Por lote de contraste, transdutor, aparelho, versão de modelo de laudo, período |
| **Indicadores confiáveis** | TAT do laudo, espera, glosa e no-show calculados a partir dos eventos, não de campos digitados |

## 5. Regras de implementação

1. **Nenhuma tabela sem os metadados da seção 1** (verificado na CI por teste de schema).
2. **Eventos imutáveis**: tabela append-only com hash encadeado; nenhuma política permite `UPDATE`/`DELETE` (RLS + papel sem permissão).
3. **Ligações tipadas** (de, para, tipo, criado_em/por), com integridade referencial; nada de "texto livre com o número da guia".
4. **Identificadores externos guardados como recebidos** (StudyUID, protocolo ANS, txid, código da NFS-e), com índice único quando o provedor garante unicidade.
5. **Hash** de todo arquivo na entrada; conferência periódica de integridade (imagens e documentos).
6. **Versão da regra aplicada** gravada em cada cálculo (preço, convênio, repasse).
7. **Retenção**: prontuário e imagens por 20 anos; financeiro e fiscal pelos prazos legais; logs técnicos pelo prazo do docs/12; tudo definido por tipo de dado.
8. **Privacidade**: o lastro também respeita perfis (docs/18): a recepção vê o operacional, não o conteúdo clínico.
9. **Testes**: para cada fluxo crítico, teste que verifica que os eventos e ligações esperados foram gravados, e que repetir a ação não duplica.

## 6. Comparação com Medware, Mediworks e Medilab

| Lastro | Medware / Medilab (o que oferecem publicamente) | VidaCor |
|---|---|---|
| Guia e autorização no agendamento | Autorizações, elegibilidade, faturamento de guias | Seção obrigatória com conferências automáticas, saldo e reserva provisória |
| Worklist, MPPS, imagens, SR | comDICOM (worklist, SR), PACS | Mais: hash por objeto, conciliação auditada, dose (RDSR) |
| Laudos e versões | Comparador de laudos, assinatura | Versões imutáveis com motivo, rastreio da IA, carimbo do tempo |
| Entrega e CD | Impressão sob demanda, CD/DVD, portal | Mais: quem retirou, procuração, acessos ao portal |
| Faturamento TISS | Guias, lotes, glosas | Mais: dossiê por atendimento, regra aplicada versionada |
| Estoque, financeiro, repasse | Estoque, contas, pagamento de médico | Mais: recall por lote, txid/NSU/NFS-e ligados |
| Desinfecção de transdutor, manutenção, dose | Não encontrado nos materiais públicos | Incluído |
| Explorador de lastro e dossiê | Não encontrado nos materiais públicos | Incluído |

> A coluna dos sistemas atuais reflete só o que foi encontrado nos sites públicos
> (docs/03); o que eles fazem internamente precisa ser confirmado com a clínica.

## 7. Pendente de validação com a clínica
- Prazo da reserva provisória sem guia (48 h?) e se vale para todos os convênios.
- Regras por operadora: validade do pedido médico, exigência de CID, prorrogação de senha.
- Quais insumos rastrear além do contraste (gel, preservativo de transdutor, materiais).
- Protocolo atual de desinfecção de transdutores e de controle de qualidade dos aparelhos.
- Quem pode retirar resultado de terceiros e com qual documento.
