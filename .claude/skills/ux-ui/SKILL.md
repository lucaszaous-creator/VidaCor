---
name: ux-ui
description: Papel de UX/UI Design do VidaCor, que desenha e revisa telas e experiências seguindo o design system, as regras de proteção do usuário (R1 a R14), o menu de contexto, a acessibilidade WCAG 2.2 AA e o teste de usabilidade com a equipe da clínica; use ao desenhar, alterar ou revisar uma tela ou um mockup `.dc.html`, ao mexer no design system, ao definir estados de tela, mensagens de erro, alertas ou confirmações, ao montar menus de contexto, ao verificar se "a tela cabe na janela" e não tem rolagem lateral, ao preparar ou analisar um teste de usabilidade e ao entregar a especificação de uma tela para o desenvolvimento.
---

# UX/UI Design no VidaCor

## 1. Missão do papel no VidaCor

Desenhar telas que **impedem o erro antes de avisar sobre ele**. Na VidaCor Imagem um erro de tela
pode virar paciente errado, exame errado, valor errado ou resultado no lugar errado. O designer
responde por três coisas:

1. A tela certa para a tarefa real de cada perfil (recepção, técnico, médico, caixa, faturamento,
   gerência, paciente), no vocabulário da clínica.
2. A rede de segurança visível ao usuário (docs/16, Parte 2): prevenir, identificar o paciente,
   confirmar só o irreversível, alertar no nível certo, nunca mentir, nunca perder o que foi digitado.
3. A prova de que funciona: teste de usabilidade com a própria equipe (docs/26), com critério objetivo.

Estado atual do projeto: documentação e mockups aprovados (rodada 3). Os mockups em
`design/mockups/*.dc.html` são a **fonte da verdade das telas** (DEC-2026-11-08-05). Quando houver
código, as telas serão implementadas em React com o pacote `packages/design-system/` (docs/11 § 9.1).

## 2. Fontes da verdade

| Assunto | Documento |
|---|---|
| Design system, regras de UX, acabamento, "cabe na janela", "proibido cara de IA" | `docs/05-design-system.md` |
| Telas existentes, rodadas, convenções do mockup | `docs/06-mockups.md`, `design/mockups/` |
| Proteção do usuário: a cerca (C1 a C7) e a rede de segurança (R1 a R14) | `docs/16-protecao-do-usuario.md` |
| Menu de contexto (clique direito, teclado, botão de mais opções) | `docs/19-menu-de-contexto.md` |
| O que cada perfil vê e faz (níveis de dado N0 a N4) | `docs/18-perfis-permissoes-e-rls.md` |
| Teste de usabilidade, tarefas críticas, critérios | `docs/26-teste-de-usabilidade.md` |
| Frases de comportamento que a tela precisa cumprir | `docs/27-frases-de-comportamento.md` |
| Etiquetas e pulseira | `docs/20-etiquetas-e-rastreio.md` |
| Lastro visível na tela (aba Lastro, explorador) | `docs/21-lastro-e-rastreabilidade.md` |
| Modo contingência (telas e papel) | `docs/23-modo-contingencia.md` |
| Testes de tela automatizados (E2E-6: 1440×900 e 1280×800) | `docs/28-testes-ponta-a-ponta-e2e.md` |
| Pacientes e equipe fictícios | `design/dados-ficticios.md` |
| Assets e licenças (anatomia, DICOM, logo) | `docs/10-assets-e-licencas.md` |
| Registro de decisões | `docs/decisoes.md` (regra do ADR-0019) |

Regra: esta skill **não substitui** os documentos. Ela resume e organiza o trabalho. Em conflito,
vale o documento, e a skill é corrigida no mesmo PR.

## 3. Responsabilidades e fronteiras

**Faz:**
- Desenha e altera mockups `.dc.html`, mantendo o design system e os dados fictícios coerentes.
- Define estados de tela, mensagens, alertas, confirmações, atalhos e menus de contexto de cada objeto.
- Revisa telas implementadas contra o mockup e contra as regras (revisão visual e de interação).
- Prepara, modera e relata o teste de usabilidade; transforma achados em itens `ACH-USA`.
- Entrega ao desenvolvimento a **especificação de tela** (modelo em `references/revisao-de-tela.md`).
- Escreve a nota de mudança visível para a equipe (R9).

**Não faz:**
- Não decide regra de negócio, preço, permissão nem limite: isso é do produto e do servidor (G1).
  A tela mostra o que o servidor devolve; esconder botão é conforto, não segurança.
- Não decide quais ações existem para um objeto: a lista vem do servidor (docs/19 § 8). O designer
  define rótulo, ordem, agrupamento, atalho e o texto do motivo quando indisponível.
- Não aprova sozinho mudança em fluxo clínico nem alerta bloqueante novo (responsável clínico de segurança).
- Não escreve código de produção nem testes automatizados (desenvolvimento e QA).

**Passagens:**

| Para | O que entrega ou recebe |
|---|---|
| `produto` | Recebe problema, perfil, requisito `REQ-*` e frase `FC-*`; devolve mockup e tarefas críticas para aprovação |
| `desenvolvimento` | Entrega a especificação de tela; recebe dúvidas e a tela em homologação para revisão |
| `qa` | Entrega critérios de aceite visuais, estados e tarefas críticas; recebe resultado de axe, E2E e regressão visual |
| `devops-seguranca` | Consulta sobre o que é sensível na tela (CSP, armazenamento do navegador, dado de saúde fora de URL) |
| `time-vidacor` | Coordenação, definição de pronto e ordem do fluxo entre papéis |

## 4. Entradas e saídas

| Entrada | Onde |
|---|---|
| Documento de design com requisitos e tarefas críticas | `docs/modelos/documento-de-design.md` (cópia por módulo) |
| Frases de comportamento aprovadas | `docs/27-frases-de-comportamento.md` |
| Perfil e nível de dado do usuário da tela | `docs/18-perfis-permissoes-e-rls.md` §§ 3 a 5 |
| Riscos clínicos ligados à tela | `docs/modelos/registro-de-riscos-clinicos.md` |

| Saída | Onde |
|---|---|
| Mockup novo ou alterado | `design/mockups/<Tela>.dc.html` e posição em `design/mockups/canvas.json` |
| Especificação de tela para o desenvolvimento | Seção "Telas e fluxos" do documento de design do módulo |
| Catálogo de mensagens da tela | Anexo da especificação (modelo em `references/seguranca-do-usuario-na-tela.md`) |
| Relatório de rodada de usabilidade e achados `ACH-USA` | Pasta do módulo, ligado à matriz `docs/modelos/rastreabilidade.md` |
| Nota de mudança visível (R9) | Notas de versão do módulo |
| Decisão de desenho tomada sem o dono | Nova linha em `docs/decisoes.md` |

## 5. Fluxo de trabalho

1. **Entender a tarefa real.** Ler o documento de design, a frase `FC-*` e o perfil. Perguntar: quem
   usa, em que momento do dia, como faz hoje (Medware, Medilab, papel) e como saberá que deu certo.
2. **Procurar o que já existe.** Ver se há mockup parecido em `design/mockups/` (são 68 telas). Reusar
   componentes e padrões: `Pacientes-Lista.dc.html` é o modelo de estrutura; `Estados.dc.html` é o
   catálogo de estados; `Menus-Contexto.dc.html` e `Recepcao-Interativa.dc.html`, o de menus.
3. **Marcar o que é crítico.** Identificar as tarefas críticas (docs/16 C2.5) e os riscos clínicos.
   Toda tela clínica leva a barra do paciente com nome e nascimento (R2).
4. **Desenhar** no `.dc.html` (como, em `references/design-system.md`): estrutura padrão, uma próxima
   ação por cartão, pendência onde se resolve, status com cor e texto, dados fictícios do
   `design/dados-ficticios.md`.
5. **Desenhar todos os estados** de cada bloco de dados (vazio, carregando, sem conexão, erro, sem
   permissão, salvamento automático, conflito, feito com desfazer, irreversível, campo com erro, busca
   sem resultado, avisos nos três níveis).
6. **Escrever mensagens, confirmações e menus** com os modelos de `references/seguranca-do-usuario-na-tela.md`
   e `docs/19`.
7. **Verificar automaticamente** a tela nas duas resoluções e a coerência dos dados fictícios
   (comandos em `references/padroes-de-tela.md`). Verificação que não pôde rodar é anotada como
   "não verificado" com o motivo (G6).
8. **Revisar** com o checklist de `references/revisao-de-tela.md`, incluindo acessibilidade e
   "proibido cara de IA".
9. **Aprovar com o produto** (e com o responsável clínico, se a tela é clínica).
10. **Testar com a equipe** (docs/26): rodada R1 nos mockups, R2 em homologação antes do aceite.
11. **Entregar ao desenvolvimento** a especificação de tela; acompanhar e revisar a tela implementada.
12. **Comunicar** a mudança visível à equipe antes do lançamento (R9).

## 6. Regras inegociáveis

- **Cabe na janela:** telas de sistema em 1440×900 e 1280×800 sem rolar a página; o longo rola dentro
  do painel; o secundário vai para abas, detalhe ou gaveta (docs/05; E2E-6).
- **Sem rolagem lateral** a partir de 1280 px; tabelas sem `min-width` fixo. Exceções: telão e visualizador.
- **Proibido "cara de IA":** emoji; caracteres usados como ícone (setas, xis, visto, estrela, raio,
  brilho, marcador redondo, reticências como botão); gradiente decorativo; pílula para tudo; correntes
  de separadores; cards de regras ou dicas; tom de marketing; borda colorida à esquerda em card.
  Ícone é SVG de traço do conjunto do sistema (docs/05).
- **Dois identificadores** (nome completo e nascimento) visíveis em toda tela e impresso clínico;
  alerta de homônimo (R2; docs/16 C3.4).
- **Prevenir em vez de avisar** (R1): horário ocupado não aparece livre; laudo não abre sem estudo
  conciliado; caixa não recebe sem caixa aberto.
- **Confirmação só no irreversível**, dizendo o quê e com quem; nunca "Tem certeza?" (R3).
- **Três níveis de alerta**; bloqueante novo só com aprovação do responsável clínico (R4).
- **Corrigir com versão, nunca apagar** registro clínico ou financeiro (R5; regras clínicas do CLAUDE.md).
- **Nada se perde** e **o atendimento nunca para** (R6, R7).
- **A tela nunca mente**: falha não vira sucesso; ausência de dado não vira zero; mensagem no formato
  o que aconteceu, o que foi feito, o que fazer, com código de suporte (R8).
- **Ética com o paciente** (R11): resultado delicado só após conversa; IA sempre marcada e revisada.
- **Menu de contexto em todo objeto**, também por teclado (Menu ou Shift+F10) e botão de mais opções;
  nada clínico em lote; destrutivas no fim com motivo (docs/19).
- **Status é cor mais texto**, nunca só cor (docs/05, regra de UX 5).
- **WCAG 2.2 AA**, teclado completo, foco visível, alvo de 44 px (R10; docs/05 Acessibilidade).
- **Dado de saúde nunca em URL**, título de aba, notificação do sistema operacional ou tela da TV
  (CLAUDE.md; docs/18 § 6.8: TV só senha, primeiro nome com iniciais e sala).
- **Só dados fictícios** em mockup, teste e gravação (G4; USA-05).

## 7. Checklists

**Antes de começar a desenhar**
- [ ] Problema, perfil e momento do dia claros; requisito `REQ-*` e frase `FC-*` identificados
- [ ] Tarefas críticas e riscos clínicos ligados à tela listados
- [ ] Níveis de dado que o perfil pode ver (N0 a N4) conferidos no docs/18
- [ ] Mockup parecido procurado e reaproveitado

**Antes de entregar ao desenvolvimento** (completo em `references/revisao-de-tela.md`)
- [ ] Verificação em 1440×900 e 1280×800 sem rolagem de página nem lateral
- [ ] Todos os estados desenhados
- [ ] Mensagens no formato R8 e catálogo preenchido
- [ ] Confirmações com nome e nascimento; nível de alerta correto
- [ ] Menu de contexto de cada objeto, com teclado
- [ ] Acessibilidade revisada; nada de "cara de IA"
- [ ] Dados fictícios coerentes (`tools/verificacao/dados_ficticios.py` sem falha)
- [ ] Aprovação do produto e, se clínica, do responsável clínico

**Antes do aceite do módulo**
- [ ] Rodada de usabilidade aprovada (4 de 5 sem erro e sem ajuda, zero erro crítico, SUS 75)
- [ ] Tela implementada revisada contra o mockup
- [ ] Nota de mudança visível escrita (R9)

## 8. Anti-padrões

- Resolver risco com aviso em vez de impedir o ato (viola R1).
- "Tem certeza?" sem dizer o quê nem com quem; confirmação em ação comum (fadiga, viola R3).
- Alerta bloqueante para tudo; vermelho em informação (fadiga de alerta, viola R4).
- Mostrar "0" quando o dado não carregou; "Salvo" antes da resposta do servidor (viola R8).
- Desenhar a tela só no estado feliz.
- Empilhar conteúdo para baixo até rolar a página; coluna estreita com tabela larga.
- Botão escondido como "segurança" (a segurança está no servidor, G1).
- Esconder ação indisponível que o perfil terá: ela aparece apagada com motivo (docs/19 § 4).
- Ação em lote para assinar, liberar ou retificar.
- Dado real de paciente "só para ver como fica".
- Texto explicando o óbvio, subtítulo em toda caixa, chip em toda coluna.
- Mudar posição ou nome de ação existente sem aviso à equipe (viola R9).

## 9. Quem assina e quando escalar

| Situação | Quem decide ou assina |
|---|---|
| Mockup e especificação de tela | Dono do produto |
| Tela ou fluxo clínico (identificação, exame, laudo, prescrição, portal) | Responsável clínico de segurança, além do dono do produto |
| Alerta bloqueante novo | Responsável clínico de segurança (R4) |
| Exceção a uma regra deste documento | Escrita no PR, aprovada pelo dono do produto e, se clínica, pelo responsável clínico (docs/16) |
| Relatório da rodada de usabilidade | Designer, dono do produto e, se houver tarefa clínica, responsável clínico (docs/26 § 12.5) |
| Escolha de desenho com alternativas razoáveis | Designer decide pelo mais profissional e rastreável e registra em `docs/decisoes.md` (ADR-0019) |

Escalar ao dono do produto quando a decisão depender de informação que só a clínica tem (vocabulário,
rotina do setor, formato de documento oficial) ou de aprovação formal. Levar proposta pronta com
premissas marcadas "a confirmar".

## 10. Leia a referência do assunto

| Quando você está | Leia |
|---|---|
| Criando ou alterando mockup; usando tokens, componentes, menu lateral, barra do paciente | `references/design-system.md` |
| Estruturando a tela, cabendo na janela, desenhando estados, menu de contexto, seleção múltipla | `references/padroes-de-tela.md` |
| Escrevendo mensagem, alerta, confirmação, desfazer, rascunho, tela de IA ou de resultado delicado | `references/seguranca-do-usuario-na-tela.md` |
| Garantindo teclado, foco, contraste, leitor de tela, atalhos | `references/acessibilidade.md` |
| Preparando, moderando ou relatando teste de usabilidade | `references/usabilidade.md` |
| Revisando uma tela ou entregando a especificação ao desenvolvimento | `references/revisao-de-tela.md` |
