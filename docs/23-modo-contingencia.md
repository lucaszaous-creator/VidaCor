# 23 · Modo contingência

> **Status:** proposta (v0.1, 08/11/2026), item **E6** do [docs/22](22-lacunas-e-roadmap.md).
> Mockup: `design/mockups/Contingencia.dc.html`. Liga-se a R7 "o atendimento nunca para"
> ([docs/16](16-protecao-do-usuario.md)), ao lastro ([docs/21](21-lastro-e-rastreabilidade.md)),
> às etiquetas ([docs/20](20-etiquetas-e-rastreio.md)) e às metas de RPO/RTO do
> [docs/11](11-arquitetura-e-infraestrutura.md) § 7.4.
> Por ser procedimento que toca identificação do paciente, exame e laudo, depende do
> parecer do **responsável clínico de segurança** ([pacote de revisão](modelos/pacote-responsavel-clinico.md)).

## 1. Objetivo

Quando o sistema, o servidor ou a rede caem, o atendimento continua no papel, com o
mesmo rigor de identificação, e tudo o que foi feito é lançado depois sem perda, sem
duplicação e com marca de origem. O papel não é improviso: é um kit preparado todo dia,
com formulários e etiquetas numerados e reservados no sistema.

## 2. Níveis de indisponibilidade

| Nível | Situação | O que acontece | Declara contingência? |
|---|---|---|---|
| **N1 · Externo** | Internet fora; servidor local e rede interna funcionando | Sistema funciona. Portal, WhatsApp, TISS, PIX, NFS-e e elegibilidade entram na lista de pendências (R7) e são reenviados pelo outbox | Não. Registra-se **ocorrência de integração** no painel. Contingência só se a falha durar mais de 4 h em dia útil e travar autorização de guias |
| **N2 · Setor** | Uma estação, impressora ou um trecho da rede fora | Setor usa outra estação; se não houver, usa o kit em papel só naquele setor | Sim, **contingência parcial**, com o setor indicado |
| **N3 · Total** | Servidor local, banco ou rede interna fora; falta de energia além do nobreak | Kit em papel em todos os setores | Sim, **contingência total** |

A troca para o servidor de contingência (RTO de 30 min, docs/11) **não substitui** a
declaração: o atendimento não espera a troca terminar.

## 3. Quando declarar

Declara-se contingência quando ocorrer **qualquer** uma das condições:

1. O núcleo não responde em duas estações de setores diferentes por **mais de 10 minutos**.
2. A TI estima volta em **mais de 15 minutos**.
3. Há paciente em risco de atraso clínico (exame com contraste em preparo, urgência), independente do tempo.
4. Falta de energia com autonomia do nobreak abaixo de 10 minutos.
5. Simulado programado (§ 10).

Na dúvida, declara-se. O custo de declarar sem necessidade é baixo (fichas que não
serão usadas voltam ao estoque numerado); o custo de não declarar é atendimento
improvisado sem numeração.

## 4. Quem declara e quem responde

| Papel | Pode declarar | Responsabilidade |
|---|---|---|
| Coordenação de atendimento (ou gerente de plantão) | Sim | **Responsável pela contingência**: comunica, distribui o kit, decide encerramento |
| Recepcionista líder do turno | Sim, na ausência da coordenação | Declara e passa a responsabilidade à coordenação assim que possível |
| TI | Não declara sozinha; informa o diagnóstico e a estimativa | Diagnóstico, troca para o servidor de contingência, conferência técnica da retomada |
| Responsável técnico médico (ou médico de plantão) | Sim, por motivo clínico | Decide sobre exames com contraste, urgências e comunicação de resultado crítico |
| Responsável clínico de segurança | Informado no mesmo dia | Revisa divergências de identificação e incidentes clínicos (docs/16 C7) |

A declaração grava: motivo, nível, setores, quem declarou, responsável, hora e o que
foi observado. Com o servidor fora, a estação grava a declaração localmente e envia ao
lastro na volta (mockup: "o aviso fica neste computador e sobe quando voltar").
Em paralelo, a declaração é anotada no **formulário F-DEC** (numerado) do kit.

Comunicação imediata: todas as estações (quando possível), ramal da coordenação, TI,
médicos do turno e grupo da equipe por telefone. Pacientes na espera são avisados
pela recepção em voz alta, sem expor dados de ninguém.

## 5. Kit diário das 06:00

### 5.1 Geração
- Tarefa agendada no servidor às **06:00** gera o kit do dia em PDF (Typst), com versão do modelo e hash SHA-256 gravados no lastro.
- Destinos: (a) cópia local cifrada em cada estação da recepção e das salas; (b) pendrive cifrado no cofre da coordenação; (c) impressão na abertura.
- Se a tarefa falhar, alerta à TI e à coordenação até 06:15; a coordenação gera manualmente pela tela Contingência.
- Agendamentos feitos depois das 06:00 atualizam a cópia local das estações a cada 15 minutos (mockup: "cópia das 06:00 com atualizações até 10:42").

### 5.2 Conteúdo

| Item | Conteúdo | Onde fica |
|---|---|---|
| **Agenda do dia por sala** (PDF) | Por sala e horário: nome completo, nascimento, exame, convênio, nº da guia e validade da senha, preparo, alertas (alergia sem detalhe, triagem pendente, contraste) | Pasta da recepção e uma folha por sala |
| **Fichas de atendimento pré-preenchidas** (F-ATD) | Uma por paciente agendado, com número de formulário, código de barras do agendamento, nome e nascimento impressos e campos para conferência | Pasta da recepção |
| **Fichas em branco numeradas** (F-ATD) | Para encaixe e paciente novo | Pasta da recepção |
| **Etiquetas de contingência numeradas** | Faixa reservada no sistema (ex.: `C-0400` a `C-0499`), código de barras e QR já impressos, campo para nome e nascimento | Gaveta da recepção |
| **Formulários numerados** | F-REC recibo manual (2 vias); F-TRI triagem de segurança; F-TCL termo de consentimento (versão vigente); F-EXA registro de exame na sala; F-COM comunicação verbal de resultado urgente; F-ENT entrega de resultado; F-DIV divergência; F-DEC declaração | Armário da coordenação |
| **Tabela de preços do particular** | Versão vigente, com o número da versão e os limites de desconto por perfil | Caixa |
| **Lista de contatos** | TI, coordenação, médicos do turno, centrais das operadoras, PSP, fornecedor de energia | Recepção e salas |
| **Instrução de uma página por setor** | O que fazer, na ordem, conforme § 7 | Cada setor |

### 5.3 Numeração e controle
- Cada formulário e etiqueta tem número único por tipo, impresso, de uma **faixa reservada** no banco (índice único por tipo e número, G2). Nenhum número é reutilizado.
- A faixa entregue ao setor fica registrada (lote de formulários, quem recebeu, data).
- Ao fim da contingência, todo número da faixa precisa estar em uma situação: **usado e lançado**, **anulado com motivo** (rasurado, perdido) ou **devolvido sem uso**. Número sem situação impede o encerramento.
- Conferência do kit até **07:00**, assinada pela coordenação (no mockup: "Conferido por Carla Mendes, 06:48"); o kit do dia anterior é recolhido e destruído em fragmentadora, com registro (contém dado pessoal).

## 6. O que funciona offline em cada estação

A cópia local é **somente leitura**, cifrada, válida só no dia e apagada na geração do
kit seguinte. Quem abre a cópia precisa ter feito login naquela estação nas últimas
12 horas e informa o PIN local; cada abertura é registrada e enviada ao lastro na volta.
Os detalhes de cifra, chave e expiração ficam em ADR próprio, a escrever antes da
construção (CONT-21).

| Estação | Funciona | Não funciona | O que fazer no lugar |
|---|---|---|---|
| **Recepção** (Tauri) | Agenda do dia (busca por nome e nascimento); ficha rápida offline gravada cifrada e enviada na volta; impressão da etiqueta de contingência na impressora local com o próximo número reservado | Cadastro, check-in completo, elegibilidade, convite do portal, TV de chamada | F-ATD; chamada por voz com primeiro nome e iniciais |
| **Caixa** | Consulta à tabela de preços impressa | Caixa integrado, PIX com QR, cartão integrado, NFS-e | Maquininha avulsa, dinheiro, F-REC |
| **Sala de exame** | Aparelho adquire e **guarda as imagens**; worklist já baixada no aparelho antes da queda continua utilizável | Worklist nova, MPPS, envio ao PACS, bipe da pulseira, baixa de contraste, triagem no sistema | F-EXA, F-TRI; dados do paciente no aparelho conforme § 7.3 |
| **Laudo** | Nada do sistema | Fila, visualizador, assinatura | Laudo fica para depois; urgência conforme § 7.4 |
| **Consultório** | Nada do sistema | Prontuário, prescrição integrada | Receita e atestado em papel timbrado numerado; lançados depois |
| **Faturamento** | Portais das operadoras pela internet, se ela estiver de pé | TISS, auditoria | Conferência de guia no portal da operadora; anotação no F-ATD |
| **Portal do paciente** (nuvem) | Laudos já publicados antes da queda | Publicação de laudos novos | Paciente é avisado que o resultado sai depois da retomada |
| **Telefones e ramais** | Funcionam | — | — |

## 7. Procedimentos por setor

### 7.1 Recepção
1. Localizar o paciente na agenda local ou impressa; pegar a F-ATD pré-preenchida (ou uma em branco para encaixe).
2. **Dois identificadores**: o paciente diz nome completo e data de nascimento; a recepcionista confere com o documento com foto e com a ficha. Marca os dois campos e assina.
3. Convênio: confere nº da guia, senha e validade na agenda impressa. Sem guia impressa, confere no portal da operadora. **Sem guia conferida, não atende como convênio** (regra do docs/21): oferece remarcação ou atendimento particular com orçamento assinado, por escolha do paciente.
4. Pulseira: imprime a etiqueta de contingência na impressora local ou preenche à mão, em letra de forma, nome completo e nascimento, na etiqueta numerada. Anota o número `C-NNNN` na F-ATD.
5. Termos: F-TCL na versão vigente, assinado em papel.
6. Encaminha à sala com a F-ATD, que acompanha o paciente.

### 7.2 Caixa
1. Preço pela tabela impressa, com a versão anotada no F-REC.
2. Desconto até o limite do perfil; acima disso, só com assinatura do gerente no F-REC. Na retomada a autorização é refeita no sistema com reautenticação (SEC-AUT-5).
3. Pagamento: cartão ou PIX pela **maquininha avulsa**, ou dinheiro. Não se usa chave PIX estática (impede a conciliação automática).
4. F-REC em duas vias: uma para o paciente, outra fica com a F-ATD. NSU da maquininha anotado.
5. NFS-e emitida depois da retomada, dentro do prazo municipal (PREM-14 do [docs/25](25-plano-do-projeto.md)).

### 7.3 Sala de exame e aparelhos
1. Conferir de novo os **dois identificadores** com o paciente, a pulseira e a F-ATD. Divergência: não inicia, chama a recepção e preenche F-DIV.
2. Triagem de segurança (contraste, gestação, função renal) no F-TRI. Resultado "precisa de aprovação médica" **bloqueia** o exame até o médico escrever a conduta, assinar e anotar a hora no F-TRI. Sem médico disponível, o exame com contraste não é feito.
3. No aparelho:
   - se o paciente está na worklist já baixada, seleciona-o normalmente;
   - se não está, digita **Patient ID e Accession Number = código da etiqueta** (`C-NNNN`), nome e nascimento exatamente como na ficha. É a única situação em que se digita dado no aparelho, e ela é tratada na conciliação (§ 8.3).
4. F-EXA: aparelho, sala, operador, início e fim, contraste (produto, lote, validade, volume, via, quem aplicou), intercorrência.
5. O aparelho guarda as imagens e **reenvia ao PACS** quando o servidor voltar (envio automático ou comando do técnico, conforme o aparelho). O inventário registra a capacidade local de cada aparelho em horas de exame; aparelho com menos de 20% de espaço não inicia exame novo.

### 7.4 Laudo
- Laudos não são feitos em papel. Ficam para a retomada e entram na fila com prazo recalculado a partir da hora da retomada, com marca "exame em contingência".
- **Urgência**: o médico vê as imagens no console do aparelho, liga para o solicitante e registra no F-COM: quem ligou, para quem, hora, o que foi dito. Na retomada o F-COM é lançado e o laudo formal é feito com prioridade.
- Nenhuma imagem de contingência entra na fila de laudo antes da conciliação (§ 8.3).

### 7.5 Entrega de resultado
- Laudo já assinado antes da queda: o paciente usa o portal (nuvem).
- Impresso já pronto antes da queda: entrega com F-ENT (quem retirou, documento, assinatura, código do envelope). Na retomada o F-ENT vira o evento "entregue" do envelope.
- Nenhum laudo novo é entregue durante a contingência.

## 8. Retomada e lançamento posterior

### 8.1 Ordem
1. TI confirma servidor, banco, rede e integrações; registra a hora da volta.
2. Coordenação muda a tela para **Retomada**. A contingência continua aberta até o checklist ficar completo.
3. Estações enviam o que gravaram localmente (declaração, fichas rápidas, aberturas da cópia local), com idempotência.
4. Lançamento das fichas, na ordem: atendimento (F-ATD), exame (F-EXA, F-TRI), caixa (F-REC), comunicação (F-COM), entrega (F-ENT).
5. Conciliação das imagens.
6. Conferência do caixa e da numeração.
7. Digitalização dos papéis.
8. Encerramento (§ 9).

### 8.2 Cada papel numerado vira registro com lastro
Todo registro lançado a partir de papel tem, além dos metadados do docs/21 § 1:

| Campo | Conteúdo |
|---|---|
| `origem` | `contingencia` |
| `contingencia_id` | Código da contingência (ex.: `CTG-2026-0007`) |
| `formulario` | Tipo e número (ex.: `F-ATD 0412`) |
| `ocorrido_em` | Hora escrita no papel (informada por quem lança) |
| `registrado_em` / `registrado_por` | Hora e usuário do lançamento |
| `atendido_por` | Quem fez o ato no papel (pode ser diferente de quem lança) |
| `digitalizacao` | PDF do papel, com hash SHA-256 |
| Marca visível | "Registrado em contingência" em toda tela e no explorador de lastro |

Regras:
- Índice único por (`contingencia_id`, tipo, número): lançar a mesma ficha duas vezes, ou em duas estações ao mesmo tempo, gera um registro só (G2, G3).
- O valor do papel não manda: o servidor recalcula preço e desconto com a tabela vigente na data do atendimento (G1). Diferença vira divergência.
- Paciente novo da ficha passa pela busca de duplicados antes de ser criado (dois identificadores); se houver candidato, abre divergência, nunca junta sozinho.
- Etiqueta de contingência lançada passa a valer como etiqueta do sistema, com o mesmo código, e entra no lastro de etiquetas (docs/20).
- Baixa de estoque do contraste é feita no lançamento do F-EXA, ligando o lote ao paciente.

### 8.3 Conciliação das imagens
- O gateway DICOM reconhece estudos com Patient ID no formato `C-NNNN` ou recebidos durante a janela da contingência e os coloca na **fila de conciliação de contingência**, nunca na fila de laudo.
- A conciliação liga o estudo ao atendimento lançado pelo mesmo código e exige conferência de **nome e nascimento** (dois identificadores) por quem executou o exame, com segunda conferência do médico quando houver qualquer diferença.
- A identificação nas imagens é corrigida pelo perfil IHE PIR, com o original guardado.
- Estudo sem ficha ou ficha sem estudo é divergência.

### 8.4 Divergências

| Tipo | Exemplo (mockup) | Quem resolve |
|---|---|---|
| Identificação | Nascimento no papel 12/04/1992, no cadastro 12/04/1993 | Quem atendeu, com segunda pessoa; responsável clínico informado se o exame já foi feito |
| Guia | Guia Unimed sem senha no papel | Faturamento, com a operadora |
| Preço | Valor cobrado diferente do recalculado | Caixa e gerente |
| Imagem | Estudo sem ficha; ficha sem estudo | Técnico da sala e médico |
| Numeração | Número da faixa sem situação | Coordenação |
| Horário | Exame no papel em horário em que a sala tinha outro paciente | Coordenação e técnico |

Toda divergência tem dono, prazo, motivo da solução e o papel digitalizado anexado.
Nada é resolvido automaticamente. Correção segue a regra geral: nova versão com motivo.

## 9. Critérios de encerramento

A contingência só é encerrada quando **todos** os itens estão cumpridos, conferidos
pelo sistema e marcados no checklist da tela:

1. Servidor, banco, rede e integrações conferidos pela TI.
2. Agenda conferida com o papel; encaixes viraram agendamentos.
3. Todas as fichas lançadas; nenhuma pendente.
4. Nenhuma divergência de **identificação** aberta. Divergências administrativas (guia, preço) podem seguir abertas com dono e prazo.
5. Todos os estudos recebidos na janela conciliados.
6. Caixa conferido: recibos manuais lançados, soma por forma de pagamento igual à maquininha e ao dinheiro.
7. Toda a numeração usada, anulada ou devolvida.
8. Papéis digitalizados e arquivados na pasta do dia.

Quem encerra: quem declarou ou a coordenação, com reautenticação. O encerramento grava
duração, número de fichas, divergências e pendências que seguiram abertas.
Prazo-alvo: encerramento em até **24 horas úteis** após a volta do servidor.

## 10. Simulado trimestral

- Um simulado por trimestre, com data no calendário da clínica (mockup: próximo em 15/01/2027). Coincide com o exercício de troca para o servidor de contingência (docs/16 C6).
- Alterna dois formatos: **prático em homologação**, com fichas e pacientes fictícios, e **ao vivo em um setor**, num turno de baixo movimento, por até 30 minutos, com aprovação da coordenação e do responsável clínico. O simulado ao vivo nunca inclui exame com contraste.
- Cada simulado mede os indicadores do § 11 e gera um relatório com ações (modelo de [pós-incidente](modelos/pos-incidente.md)). Ação vira teste, regra ou treinamento.
- Contingência real conta como simulado do trimestre só se tiver relatório completo.

## 11. Indicadores

| Indicador | Meta inicial |
|---|---|
| Kit gerado e conferido até 07:00 | 100% dos dias de funcionamento |
| Tempo entre a falha detectada e a declaração | até 15 min |
| Fichas lançadas em até 24 h úteis | 100% |
| Divergências por 100 fichas | Acompanhar; meta definida após 4 trimestres |
| Divergências de identificação abertas no encerramento | 0 |
| Números da faixa sem situação no encerramento | 0 |
| Estudos de contingência sem conciliação após 24 h | 0 |
| Tempo da volta do servidor ao encerramento | até 24 h úteis; no simulado, até 60 min |
| Simulados realizados no ano | 4 |

Os indicadores são calculados a partir dos eventos do lastro, não de campos digitados.

## 12. Papéis e responsabilidades

| Atividade | Coordenação | Recepção | Caixa | Técnico | Médico | Faturamento | TI | Resp. clínico |
|---|---|---|---|---|---|---|---|---|
| Conferir kit diário | R | A | — | — | — | — | I | — |
| Declarar | R | R (na ausência) | — | — | R (motivo clínico) | — | C | I |
| Atender no papel | A | R | R | R | R | C | — | — |
| Exame com contraste em contingência | I | — | — | R | A | — | — | I |
| Comunicação verbal de urgência | I | — | — | — | R | — | — | I |
| Retomada técnica | I | — | — | — | — | — | R | — |
| Lançar fichas | A | R | R | R | R | R | — | — |
| Conciliar imagens | I | — | — | R | A | — | C | I |
| Resolver divergência de identificação | A | R | — | R | R | — | — | C |
| Encerrar | R | — | — | — | — | — | C | I |
| Simulado e relatório | R | C | C | C | C | C | R | A |

Legenda: R executa; A aprova; C é consultado; I é informado.

## 13. Testes obrigatórios (G4)

Todos pela rota HTTP real, com PostgreSQL e Orthanc reais (Testcontainers). As frases
estão em [docs/27](27-frases-de-comportamento.md) (FC-CTG-*) e aguardam aprovação.

| Teste | O que prova |
|---|---|
| Geração do kit | Às 06:00 gera agenda por sala, fichas e reserva de faixa; hash e versão do modelo no lastro |
| Reserva de faixa simultânea | Duas reservas ao mesmo tempo não recebem números sobrepostos |
| Lançamento repetido | A mesma ficha lançada duas vezes (mesma `Idempotency-Key` ou mesmo número) gera um registro |
| Lançamento simultâneo | Duas estações lançam a mesma ficha ao mesmo tempo: um registro, a outra recebe 409 com o registro existente |
| Número fora da faixa | Ficha com número não reservado é recusada (422) |
| Marca de contingência | Registro lançado tem `origem = contingencia`, formulário, `ocorrido_em`, `atendido_por` e digitalização com hash |
| Preço do servidor | Recibo com valor diferente do recalculado gera divergência; o valor gravado é o do servidor |
| Paciente novo com possível duplicado | Abre divergência; nenhum cadastro é unido |
| Estudo `C-NNNN` | Vai para a fila de conciliação de contingência e não aparece na fila de laudo |
| Conciliação sem dois identificadores | Recusada |
| Encerramento com pendência | Recusado enquanto houver ficha pendente, divergência de identificação, estudo sem conciliação ou número sem situação |
| Declaração offline | Declaração gravada na estação sobe uma vez só, mesmo reenviada |
| Cópia local expirada | Cópia do dia anterior não abre |
| Acesso indevido | Perfil Caixa não resolve divergência de identificação nem vê F-EXA; usuário de outra unidade não vê a contingência |
| Lançamento após encerramento | Recusado; exige reabertura com motivo |

## 14. Requisitos

| ID | Requisito | Critério de aceite |
|---|---|---|
| CONT-01 | Contingência tem três níveis (externo, setor, total) e é declarada pela tela ou pelo F-DEC | Declaração grava motivo, nível, setores, quem declarou, responsável e hora |
| CONT-02 | Só os perfis do § 4 declaram | Outro perfil recebe 403 |
| CONT-03 | Declaração feita sem servidor fica na estação e sobe ao voltar, sem duplicar | Teste de declaração offline |
| CONT-04 | Kit gerado automaticamente às 06:00, com alerta se falhar até 06:15 | Teste de geração; alerta testado |
| CONT-05 | Kit contém os itens do § 5.2 | Conferência por modelo versionado |
| CONT-06 | Faixas de numeração reservadas no banco com índice único por tipo e número | Teste de reserva simultânea |
| CONT-07 | Cópia local da agenda, cifrada, somente leitura, válida no dia | Teste de expiração; abertura registrada |
| CONT-08 | Estações mostram o que funciona e o que não funciona (§ 6) | Usabilidade da tela Contingência |
| CONT-09 | Dois identificadores conferidos e assinados na F-ATD e na sala | Campo obrigatório no lançamento |
| CONT-10 | Convênio sem guia conferida não é lançado como convênio | Lançamento recusado ou convertido com motivo |
| CONT-11 | Triagem "precisa de aprovação médica" exige conduta do médico no lançamento do F-TRI | Lançamento sem conduta recusado |
| CONT-12 | Lançamento grava `origem`, `contingencia_id`, formulário, `ocorrido_em`, `atendido_por` e digitalização com hash | Teste de marca de contingência |
| CONT-13 | Lançar a mesma ficha não duplica | Testes de repetição e simultaneidade |
| CONT-14 | Servidor recalcula preço e desconto no lançamento; diferença vira divergência | Teste de preço do servidor |
| CONT-15 | Desconto acima do limite exige autorização no sistema com reautenticação | Teste de desconto em contingência |
| CONT-16 | Estudos de contingência vão para fila própria e só seguem ao laudo após conciliação com dois identificadores e correção PIR | Testes de estudo `C-NNNN` |
| CONT-17 | Divergências têm tipo, dono, prazo, motivo e papel anexado; nenhuma resolvida automaticamente | Teste de divergência |
| CONT-18 | Encerramento só com os critérios do § 9, por quem declarou ou pela coordenação, com reautenticação | Teste de encerramento com pendência |
| CONT-19 | Toda a numeração reservada termina usada, anulada com motivo ou devolvida | Relatório de numeração no encerramento |
| CONT-20 | Indicadores do § 11 calculados a partir do lastro | Painel conferido contra eventos |
| CONT-21 | Cópia local definida em ADR (cifra, chave, PIN, expiração, quem acessa) antes de construir | ADR aprovado |
| CONT-22 | Simulado trimestral registrado com relatório e ações | Quatro relatórios por ano |
| CONT-23 | Lastro de contingência respeita perfis (docs/18): recepção vê o operacional, não o conteúdo clínico | Teste de acesso indevido |
| CONT-24 | Registros de contingência aparecem com a marca "Registrado em contingência" em toda tela e no explorador de lastro | Usabilidade e teste de tela |

## 15. Pendente de validação com a clínica
- Lista de aparelhos que guardam e reenviam imagens sozinhos e capacidade local de cada um (inventário do docs/09).
- Se a maquininha avulsa aceita PIX.
- Prazo de emissão da NFS-e no município da clínica.
- Quem é a coordenação de cada turno e quem substitui.
- Local físico do kit e do cofre.
