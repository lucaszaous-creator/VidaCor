# 26 · Teste de usabilidade com a equipe da clínica

> **Status:** proposta (v0.1, 08/11/2026). Cumpre o docs/16 C4 (teste de usabilidade
> com a equipe nas tarefas críticas, IEC 62366-1) e o critério de aceite comum do
> [docs/25](25-plano-do-projeto.md) § 3.2. Mockups em `design/mockups/`, em especial
> `Fluxo-Completo.dc.html` (agendar, check-in, sala, laudo e portal, com lastro ao lado).

## 1. Objetivo

Verificar, antes de construir e depois a cada módulo, se cada perfil da clínica consegue
fazer as **tarefas críticas** sem erro e sem ajuda, e encontrar o que na tela leva a erro.
Tarefa crítica é a que, feita errado, causa dano: identificar paciente, executar exame,
laudar, cobrar, faturar, publicar resultado (docs/16 C2).

## 2. Rodadas

| Rodada | Quando | Material | Tipo |
|---|---|---|---|
| R1 | Fase 0 | Mockups navegáveis (canvas, modo Play) | Formativo: encontra problemas de desenho |
| R2 em diante | Antes do aceite de cada módulo | Homologação com dados fictícios | Formativo e de aceite (docs/25 § 3.2) |
| Final | Antes da virada | Homologação com fluxo completo | Aceite das tarefas críticas de todos os perfis |

Este roteiro serve às três; muda só o material e a versão registrada.

## 3. Participantes

- **Cinco participantes por perfil**, da própria equipe, que façam a tarefa no dia a dia: recepção, técnico de sala (ou enfermagem), médico, caixa, faturamento.
- Quem participou do desenho das telas não participa como usuário.
- Mistura de tempo de casa e de familiaridade com computador.
- Participação voluntária, em horário de trabalho, sem prejuízo ao atendimento (agenda combinada com a coordenação).

## 4. Papéis na sessão

| Papel | Faz |
|---|---|
| Moderador | Lê o cenário, não ajuda, não explica a tela, pede para pensar em voz alta |
| Anotador | Preenche o formulário (§ 8): tempo, caminho, erros, pedidos de ajuda, falas |
| Observador clínico (sessões de técnico e médico) | Indica se um erro teria consequência clínica; não interfere |

## 5. Preparação

1. Registrar a versão: commit do repositório **ou**, se não houver controle de versão disponível, SHA-256 dos arquivos `.dc.html` usados e o endereço do canvas.
2. Dados **somente fictícios** (os nomes dos mockups: Sônia Maria Castro, Antônio Carlos Ribeiro, Juliana Sampaio Rocha e outros). Nunca dado real de paciente (G4).
3. Estação parecida com a real: monitor de 1280 × 800 ou 1440 × 900, teclado e mouse; leitor de código de barras quando a tarefa pedir bipe.
4. Sala reservada, 45 a 60 minutos por participante.
5. Gravação de tela e áudio só com consentimento (§ 7); sem gravação de rosto.

## 6. Roteiro da sessão

1. **Abertura (5 min):** explicar que se testa a tela, não a pessoa; que não há resposta certa; que pode parar quando quiser. Colher o consentimento.
2. **Aquecimento (3 min):** navegar livremente pelo menu.
3. **Tarefas (30 a 40 min):** uma por vez, lidas pelo moderador a partir do cartão do cenário. O participante diz "terminei" quando acha que acabou.
4. **Questionário SUS (5 min)** (§ 9.4).
5. **Conversa final (5 min):** o que foi mais difícil, o que faria errado num dia corrido, o que falta.

## 7. Consentimento

Termo de participação, assinado antes da sessão, com:
- objetivo (avaliar a tela, não o desempenho do funcionário);
- participação voluntária e direito de parar a qualquer momento sem consequência;
- o resultado individual **não** é usado em avaliação de desempenho, promoção ou repasse;
- gravação de tela e áudio opcional, marcada à parte; guardada com acesso restrito e apagada 90 dias após o relatório da rodada, com registro;
- identificação apenas por código (`P-REC-03`); a tabela que liga código e nome fica com a coordenação, separada dos resultados, e é destruída com a gravação;
- dados tratados conforme a LGPD, com contato do encarregado (DPO).

Não é pesquisa científica. Se a clínica quiser publicar os resultados, avalia antes a
necessidade de submissão a comitê de ética.

## 8. Formulário de registro (por sessão)

| Campo | Conteúdo |
|---|---|
| ID da sessão | `USA-AAAA-RN-NNN` (ano, rodada, número), ex.: `USA-2026-R1-014` |
| Data, início e fim | |
| Rodada | R1, R2, ..., Final |
| Material e versão | Mockup ou homologação; commit ou SHA-256 dos arquivos; versão do sistema em homologação |
| Participante | Código anonimizado (`P-REC-03`), perfil, tempo de casa em faixas (até 1 ano, 1 a 5, mais de 5) |
| Moderador, anotador, observador | Nomes |
| Consentimento | Sim; gravação sim ou não |
| Por tarefa | ID da tarefa; concluída sem ajuda, com ajuda ou não concluída; tempo; número de erros; erros críticos com descrição; pedidos de ajuda; caminho seguido; falas relevantes |
| SUS | 10 respostas e pontuação |
| Observações | |

O formulário é preenchido em planilha ou formulário eletrônico com acesso restrito e
guardado junto ao relatório da rodada.

## 9. Tarefas críticas por perfil

O tempo-alvo é referência para a rodada R1 e é recalibrado com os tempos medidos.
**Erro crítico** é o que levaria a dano: paciente errado, exame errado, valor errado,
resultado no lugar errado.

### 9.1 Recepção

| ID | Tarefa (cenário lido ao participante) | Mockup | Tempo-alvo | Erro crítico |
|---|---|---|---|---|
| T-REC-01 | Agende um ecodoppler para Antônio Carlos Ribeiro, convênio Unimed, com a guia que ele trouxe | Fluxo-Completo (etapa 1), Agendamento-Guia | 3 min | Agendar sem conferir a guia; aceitar guia vencida |
| T-REC-02 | Paciente sem guia quer agendar; deixe o horário guardado | Agendamento-Guia | 2 min | Agendar como confirmado sem guia |
| T-REC-03 | Faça o check-in de Antônio, que chegou agora | Fluxo-Completo (etapa 2), Recepcao | 2 min | Concluir sem conferir nome e nascimento |
| T-REC-04 | Chegaram duas pacientes com nome parecido; faça o check-in da que está à sua frente | Recepcao | 2 min | Check-in na paciente errada |
| T-REC-05 | A pulseira rasgou; providencie outra | Etiquetas | 1 min | Reimprimir sem invalidar a anterior |
| T-REC-06 | O sistema caiu; atenda o próximo paciente e, quando voltar, lance a ficha | Contingencia | 5 min | Ficha sem número ou sem dois identificadores |

### 9.2 Técnico de sala

| ID | Tarefa | Mockup | Tempo-alvo | Erro crítico |
|---|---|---|---|---|
| T-TEC-01 | Chame o próximo paciente, confirme que é ele e inicie o exame | Fluxo-Completo (etapa 3), Sala-Exame, Barra-Paciente | 2 min | Iniciar sem bipar a pulseira |
| T-TEC-02 | A pulseira bipada não é do paciente desta sala; resolva | Fluxo-Completo com "erro" ligado, Etiquetas | 2 min | Contornar o bloqueio |
| T-TEC-03 | Paciente vai fazer TC com contraste; faça a triagem | Triagem | 4 min | Liberar exame que precisa de aprovação médica |
| T-TEC-04 | Registre o contraste aplicado | Sala-Exame | 1 min | Lote errado ou não registrado |
| T-TEC-05 | Finalize o exame e envie para laudo com prioridade urgente | Sala-Exame | 1 min | Enviar com imagens de outro paciente |

### 9.3 Médico

| ID | Tarefa | Mockup | Tempo-alvo | Erro crítico |
|---|---|---|---|---|
| T-MED-01 | Abra o próximo laudo da sua fila, confira as medidas e assine | Fluxo-Completo (etapa 4), Laudo | 5 min | Assinar laudo de outro paciente; assinar sem conferir medidas |
| T-MED-02 | Faça o laudo estruturado da tireoide com o nódulo que cresceu | Laudo-Estruturado | 6 min | Categoria diferente da calculada sem perceber |
| T-MED-03 | Corrija um laudo já assinado | Laudo | 3 min | Achar que alterou o original |
| T-MED-04 | Resultado delicado: impeça que vá direto ao portal | Laudo, Fluxo-Laudos | 2 min | Liberar no portal sem conversa |
| T-MED-05 | Registre a conduta da triagem que pede sua aprovação | Triagem | 2 min | Aprovar paciente errado |
| T-MED-06 | Compare com o exame externo que a paciente trouxe | Exame-Externo | 3 min | Tratar o externo como laudo da VidaCor |

### 9.4 Caixa

| ID | Tarefa | Mockup | Tempo-alvo | Erro crítico |
|---|---|---|---|---|
| T-CXA-01 | Abra o caixa com o fundo de troco | Caixa | 1 min | — |
| T-CXA-02 | Receba o particular de Sônia Maria Castro, parte no PIX e parte no cartão | Caixa | 3 min | Valor diferente do devido |
| T-CXA-03 | O paciente pede desconto acima do seu limite | Caixa, Orcamento | 2 min | Conceder sem autorização |
| T-CXA-04 | Estorne um recebimento com motivo | Caixa | 2 min | Estornar recebimento errado |
| T-CXA-05 | Feche o caixa com diferença de R$ 10 | Caixa | 3 min | Fechar escondendo a diferença |

### 9.5 Faturamento

| ID | Tarefa | Mockup | Tempo-alvo | Erro crítico |
|---|---|---|---|---|
| T-FAT-01 | Peça a autorização de um exame pelo sistema | Autorizacao | 3 min | Pedido com procedimento diferente do agendado |
| T-FAT-02 | Resolva as pendências da auditoria e monte o lote da Unimed | Auditoria-Faturamento, Faturamento | 6 min | Incluir guia com erro bloqueante |
| T-FAT-03 | Registre uma glosa e prepare o recurso | Faturamento | 4 min | Recurso na guia errada |
| T-FAT-04 | Encontre toda a história do atendimento a partir do número da guia | Lastro | 2 min | — |

## 10. Métricas

| Métrica | Como mede |
|---|---|
| **Sucesso** | Concluída sem ajuda, com ajuda ou não concluída, por tarefa |
| **Tempo** | Do fim da leitura do cenário até "terminei" |
| **Erros** | Ações que se afastam do caminho correto; separados em críticos e não críticos |
| **Pedidos de ajuda** | Quantas vezes o participante pede orientação |
| **SUS** (System Usability Scale) | Questionário de 10 itens, escala de 1 a 5, pontuação de 0 a 100, em versão em português |

## 11. Critérios de aprovação

Por tarefa crítica:
1. Pelo menos **4 de 5** participantes do perfil concluem **sem erro e sem ajuda** (docs/16 C4).
2. **Nenhum erro crítico** em nenhum participante. Um erro crítico **bloqueia** a aprovação da tarefa até a tela mudar e a tarefa ser retestada.
3. Tempo mediano até 1,5 vez o tempo-alvo.

Por perfil:
4. SUS médio de pelo menos **75**.

Rodada aprovada: todas as tarefas e perfis aprovados. Tarefa reprovada volta a ser
testada com **outros** participantes após a correção.

## 12. O que fazer com os achados

1. Cada achado vira item rastreável `ACH-USA-AAAA-NNN` com: sessão de origem, tarefa, descrição do que aconteceu, gravidade, tela e versão.
2. Gravidade:

| Nível | Critério | Tratamento |
|---|---|---|
| Crítico | Erro crítico ocorreu ou poderia ocorrer | Bloqueia o módulo; vira risco no [registro de riscos clínicos](modelos/registro-de-riscos-clinicos.md) quando clínico; reteste obrigatório |
| Alto | Tarefa não concluída ou concluída só com ajuda | Corrigido antes do aceite |
| Médio | Atraso ou hesitação relevante | Planejado na próxima iteração |
| Baixo | Incômodo | Lista de melhorias |

3. Cada item aponta para o requisito (`REQ-…`) ou cria um, e para o teste que o comprova (matriz de [rastreabilidade](modelos/rastreabilidade.md)).
4. Achado de tarefa clínica é enviado ao responsável clínico de segurança com o [pacote de revisão](modelos/pacote-responsavel-clinico.md).
5. Relatório da rodada: participantes por perfil, resultado por tarefa, SUS, achados com gravidade e dono, decisão (aprovado ou não). Assinam o designer, o dono do produto e, se houver tarefa clínica, o responsável clínico.
6. Nenhum achado é descartado sem motivo registrado.

## 13. Requisitos

| ID | Requisito | Critério de aceite |
|---|---|---|
| USA-01 | Toda tarefa crítica de cada perfil tem cenário, tempo-alvo e definição de erro crítico | Tabelas do § 9 revisadas pelo dono do produto |
| USA-02 | Cinco participantes por perfil, anonimizados por código | Formulários da rodada |
| USA-03 | Toda sessão registra a versão do material (commit ou hash) | Campo preenchido em 100% das sessões |
| USA-04 | Consentimento assinado antes de cada sessão | Termos arquivados |
| USA-05 | Só dados fictícios | Conferência do material antes da rodada |
| USA-06 | Critérios do § 11 aplicados; erro crítico bloqueia | Relatório da rodada |
| USA-07 | Todo achado vira item `ACH-USA` ligado a requisito e teste | Matriz de rastreabilidade |
| USA-08 | Gravações apagadas 90 dias após o relatório, com registro | Registro de eliminação |
