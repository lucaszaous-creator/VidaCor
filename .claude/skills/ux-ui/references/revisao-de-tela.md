# Revisão de tela e passagem para o desenvolvimento

Use este checklist ao terminar um mockup (antes de pedir aprovação) e ao revisar a tela
implementada em homologação. Item não atendido sem exceção aprovada bloqueia a entrega.

## 1. Checklist de revisão

**Tarefa e conteúdo**
- [ ] A tela resolve a tarefa do perfil, no vocabulário da clínica
- [ ] Uma próxima ação por cartão; o resto em "mais opções"
- [ ] A pendência aparece onde se resolve
- [ ] Só os níveis de dado que o perfil pode ver (docs/18); N3 e N4 só na tela do ato
- [ ] Dados fictícios coerentes com `design/dados-ficticios.md`

**Layout**
- [ ] 1440×900 e 1280×800 sem rolar a página (`tools/verificacao/verifica.sh` sem `sh` nem `cortado`)
- [ ] Sem rolagem lateral (`ladoalado` vazio); tabela sem `min-width` fixo
- [ ] Longo rola dentro do painel com cabeçalho fixo; secundário em abas, detalhe ou gaveta
- [ ] Nenhum bloco abaixo de cerca de 340 px; nomes em uma linha; células com até 2 linhas
- [ ] Funciona em 1366×768 e zoom de 150%

**Segurança do usuário**
- [ ] Barra do paciente com nome e nascimento em tela clínica; alerta de homônimo previsto
- [ ] Erro prevenido onde possível (R1)
- [ ] Confirmação só em ato irreversível, com o quê e com quem (R3)
- [ ] Nível de alerta correto; nenhum bloqueante novo sem aprovação clínica (R4)
- [ ] Correção por nova versão com motivo; desfazer só em ação comum (R5)
- [ ] Rascunho automático e reenvio sem perda (R6); pendências quando algo externo falha (R7)
- [ ] Mensagens no formato R8, com código de suporte; a tela nunca mente
- [ ] Resultado delicado retido; IA marcada e revisada (R11)

**Todos os estados** (lista em `references/padroes-de-tela.md` § 4)
- [ ] Vazio, carregando, sem conexão, erro, sem permissão, salvamento, conflito, feito com desfazer,
      irreversível, campo com erro, busca sem resultado, avisos nos três níveis

**Menu de contexto**
- [ ] Todo objeto com clique direito, Shift+F10, tecla Menu e botão de mais opções
- [ ] Cabeçalho com o objeto; ordem fixa; destrutivas no fim com motivo
- [ ] Indisponível apagado com motivo; inexistente para o perfil não aparece
- [ ] Nada clínico em lote

**Acessibilidade** (`references/acessibilidade.md` § 8)
- [ ] Contraste AA, teclado completo, foco visível, alvos de 44 px, `aria-label` nos ícones

**Proibido "cara de IA"** (docs/05)
- [ ] Nenhum emoji
- [ ] Nenhum caractere usado como ícone (setas, xis, visto, triângulo de play, círculo, estrela,
      raio, brilho, aspas angulares, reticências, marcador); ícone é SVG de traço do conjunto
- [ ] Nenhum ícone de raio, brilho, estrela mágica, sol ou varinha
- [ ] Nenhum gradiente decorativo (só o esqueleto e o fundo do visualizador anatômico)
- [ ] Chip só para situação, no máximo um por linha de lista
- [ ] No máximo um separador por linha; sem correntes de pontos
- [ ] Sem maiúsculas espaçadas fora de cabeçalho de tabela; sem quadradinhos numerados de enfeite
- [ ] Sem cards de "Regras", "Dicas", "Como funciona"; sem texto explicando o óbvio; sem tom de
      marketing; sem rodapé de "mockup"
- [ ] Sem borda colorida à esquerda em card, sem sombra pesada, sem subtítulo em toda caixa

**Rastreio**
- [ ] Requisito `REQ-*`, frase `FC-*` e tarefa crítica `T-*` ligados
- [ ] Decisão de desenho relevante registrada em `docs/decisoes.md`

## 2. Revisão da tela implementada

Compare lado a lado com o mockup aprovado, nas duas resoluções:
- Mesmos componentes, tokens, textos, ordem e atalhos.
- Ações do menu vindas do servidor (a tela não monta a lista sozinha, docs/19 § 8).
- Estados reais provocados em homologação (derrubar a rede, perfil sem permissão, busca vazia).
- Resultado de axe e do teste E2E de layout (E2E-6) anexados pelo QA.
- Diferença encontrada vira item no PR (não bloqueia) ou bug (se viola regra deste checklist).

## 3. Modelo: especificação de tela para o desenvolvimento

```
Especificação de tela: <Nome da tela>
Mockup aprovado: design/mockups/<Tela>.dc.html (commit <sha>)    Aprovado por: <nome, data>
Módulo: <dominio-*>    Perfis: <Recepção, ...>    Clínica? sim/não (parecer: <id>)
Requisitos: REQ-XXX-001, ...    Frases: FC-XXX-01, ...    Tarefas críticas: T-XXX-01, ...
Riscos clínicos: RC-001, ...

1. Objetivo da tela (uma frase, do ponto de vista do usuário)

2. Estrutura
   Cabeçalho: <título, contador, busca, ação principal>
   Abas: <lista>
   Painéis: <principal, detalhe, gaveta>; o que rola em cada um

3. Dados exibidos (por bloco)
   | Bloco | Campo | Origem (recurso da API) | Nível de dado (N0-N4) | Formato | Vazio |

4. Ações
   | Ação | Onde (botão, menu, atalho) | Rota chamada | Permissão do ato | Confirmação | Idempotency-Key? |
   A lista do menu de contexto vem do servidor para cada objeto (id, rótulo, atalho, estado,
   motivo, perigo, submenu).

5. Estados (para cada bloco): vazio, carregando, sem conexão, erro, sem permissão, salvamento,
   conflito, feito com desfazer, irreversível, campo com erro, busca sem resultado, avisos

6. Mensagens: catálogo anexo (códigos estáveis iguais aos do Problem Details)

7. Teclado e atalhos: <ordem de foco, atalhos, Shift+F10, Esc>

8. Tempo real: eventos que atualizam a tela (ex.: EstudoRecebido) e como aparecem

9. Critérios de aceite visuais
   - 1440×900 e 1280×800 sem rolagem de página nem lateral (E2E-6)
   - Igual ao mockup em componentes, textos e ordem
   - axe sem violação AA; tarefa crítica só com teclado
   - Dois identificadores visíveis (se clínica)

10. Fora do escopo desta tela
```
