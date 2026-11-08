# ADR-0021 · Design system próprio a partir dos mockups, com React Aria Components

- **Status:** aceito (aprovado pelo dono do produto em 2026-11-08)
- **Data:** 2026-11-08

## Contexto
As telas precisam sair fiéis aos mockups aprovados (`design/mockups/*.dc.html`), que são HTML com a folha
`design/vc10.css` (tokens de cor, raio, sombra e classes de componente) e ícones SVG de traço (24 px, traço 2).
O ADR-0005 fixou React, TypeScript e Vite na web e Tauri no desktop. Faltava decidir como estilizar e de onde
vem o comportamento acessível (teclado, foco, leitor de tela, menus, tabelas, datas em pt-BR) exigido no
docs/05, no docs/16 R10 e no docs/19. WPF foi descartado pelo dono do produto.

## Decisão
- **Estilo:** pacote próprio `packages/design-system/`, derivado do `design/vc10.css`: tokens viram variáveis CSS
  e cada classe vira componente React (`Card`, `Tabela`, `Abas`, `Chip`, `Botao`, `Campo`, `MenuLateral`,
  `BarraDoPaciente`, estados de tela). Sem biblioteca com visual próprio.
- **Comportamento acessível:** React Aria Components (Adobe), sem estilo próprio, estilizado com os tokens.
- **Ícones:** SVG de traço do mesmo padrão dos mockups, num único conjunto do sistema.
- **Fidelidade:** comparação automática de imagem de cada tela em 1440×900 e 1280×800 contra o mockup, além da
  verificação "a tela cabe na janela" (docs/05).

## Alternativas
- Radix UI: bom, porém com menos suporte a datas e números localizados e a tabelas navegáveis.
- Material UI, Ant Design, Bootstrap: impõem visual próprio; afastariam as telas dos mockups.
- Tailwind: exigiria reescrever todas as classes dos mockups.
- WPF: descartado pelo dono do produto; não roda no navegador nem no portal.

## Consequências
Uma só fonte de estilo (mockup e sistema), acessibilidade madura, sem dependência de visual de terceiros.
Como desfazer: novo ADR que substitua este; os componentes do pacote isolam a troca da biblioteca de comportamento.
