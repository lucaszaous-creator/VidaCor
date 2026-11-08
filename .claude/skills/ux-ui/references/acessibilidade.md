# Acessibilidade

Fontes: `docs/05-design-system.md` (Acessibilidade e menu lateral), `docs/16-protecao-do-usuario.md`
R10 e C4.4, `docs/14-regras-de-arquitetura-e-engenharia.md` B9, `docs/19-menu-de-contexto.md`,
critério de aceite 7 do `docs/25-plano-do-projeto.md` § 3.2.

Requisito, não melhoria: **WCAG 2.2 nível AA** em toda tela, verificada por axe na CI e por teste
manual de teclado e leitor de tela antes do aceite do módulo.

## 1. Contraste e cor

- Texto normal com contraste mínimo de 4,5:1; texto grande e ícones essenciais 3:1.
- `--mute` (`#5B6474`) foi escolhido para passar 4,5:1 no branco; não usar cinza mais claro para texto.
- Cor nunca é a única pista: status com cor e texto (e símbolo SVG); item do menu com nome ou dica.
- Verde e vermelho não são o único contraste quando a diferença importa: usar azul e laranja (docs/05).
- Alergia em vermelho cheio com texto branco e a palavra "Alergia" escrita.

## 2. Teclado

- Toda tela utilizável só pelo teclado, em ordem lógica (cabeçalho, abas, lista, detalhe, ações).
- Foco sempre visível e nunca escondido por cabeçalho fixo (WCAG 2.2, foco não obscurecido).
- Atalhos documentados no docs/05: Ctrl+K busca, F2 check-in, F9 ditado.
- Menu de contexto: tecla Menu ou Shift+F10 abre; setas navegam; Enter executa; Esc fecha; números
  escolhem sugestões de submenu (docs/19).
- Janelas e gavetas: foco vai para dentro ao abrir, fica preso enquanto aberta e volta ao elemento
  de origem ao fechar; Esc fecha (chat flutuante incluído).
- Atalho novo só se não conflitar com leitor de tela e navegador; listar no cartão de atalhos da tela.

## 3. Alvos e conforto

- Alvo de toque e clique com pelo menos 44 px (acima do mínimo do WCAG 2.2, decisão do docs/05).
- Pensado para 1366×768 e zoom de 150%: o conteúdo reflui, nada é cortado, nenhuma ação some.
- Portal e app com tamanho de letra ajustável (R10).
- Nada de ação que dependa só de arrastar: arrastar agendamento tem alternativa por menu ("Recortar
  para mover", "Remarcar") (WCAG 2.2, movimentos de arrastar).
- Tempo limite de sessão avisa antes e permite estender, sem perder o rascunho (R6).

## 4. Controles reais e leitor de tela

- Usar `button`, `a`, `label`, `input`; nada de `div` clicável.
- Botão só com ícone tem `aria-label` (o botão de mais opções diz "Mais opções de Sônia Maria Castro").
- Texto só para leitor de tela com a classe `sr` (como o rótulo da busca em `Pacientes-Lista`).
- Abas com `aria-current` ou papel de aba; linha selecionada com `aria-selected`.
- Menu lateral recolhível com `aria-expanded` e rótulo.
- Mudanças vivas (paciente chegou, imagem recebida, senha chamada) anunciadas por região viva
  educada; alerta bloqueante por região assertiva.
- Tabelas com cabeçalho real (`th`) e cabeçalho fixo ao rolar.
- Campo com erro: mensagem ligada ao campo, anunciada, com o valor mantido.
- Imagem clínica com texto alternativo descritivo do que é (série, incidência), sem diagnóstico.

## 5. Movimento e som

- Sem animação quando o sistema pede movimento reduzido (menu lateral, esqueleto, transições).
- Nada pisca mais de três vezes por segundo.
- Mensagem urgente do chat toca som e também tem indicação visual e textual.

## 6. Linguagem

- Português claro, vocabulário da clínica, frases curtas.
- Erros dizem o que fazer (R8). Rótulos dizem a ação ("Assinar e liberar", não "Confirmar").
- Siglas na primeira ocorrência por extenso quando o perfil não as usa no dia a dia.

## 7. Como verificar

| Verificação | Quem | Quando |
|---|---|---|
| axe automático sem violação AA | QA (CI) | A cada PR com tela |
| Percorrer a tarefa crítica só com teclado | Designer e QA | Antes de entregar e no aceite |
| Leitor de tela (NVDA no Windows, padrão das estações) nas tarefas críticas | Designer e QA | Antes do aceite do módulo |
| Zoom de 150% e 1366×768 | Designer | Antes de entregar |
| Contraste dos tokens novos | Designer | Ao criar token |

Leitor de tela de referência: NVDA, por as estações serem Windows (docs/13); a confirmar com a TI
da clínica.

## 8. Checklist rápido

- [ ] Contraste AA em todo texto e ícone essencial
- [ ] Status e alerta com texto, não só cor
- [ ] Tarefa crítica completa só com teclado; foco visível e não obscurecido
- [ ] Menu de contexto por Shift+F10 e tecla Menu
- [ ] Alvos de pelo menos 44 px
- [ ] Botões só com ícone com `aria-label`
- [ ] Regiões vivas para eventos em tempo real
- [ ] Sem animação com movimento reduzido
- [ ] Funciona em zoom de 150% e 1366×768
- [ ] Arrastar tem alternativa sem arrastar
