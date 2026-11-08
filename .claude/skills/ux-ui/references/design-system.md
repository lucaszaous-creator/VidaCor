# Design system VidaCor

Fonte: `docs/05-design-system.md`. Folha de estilos dos mockups: `design/vc10.css` (o docs/05 também
cita `design/assets/vidacor.css` como origem do design system). Referências visuais aprovadas pelo
cliente: `design/referencias/`. Em conflito, vale o docs/05.

## 1. Marca

- Logo: "V" azul-marinho com cinza, VIDACOR (VIDA marinho, COR azul), IMAGEM e o slogan
  "Tecnologia que cuida da sua vida". Recortes em `design/assets/marca/`.
- Cores do logo: marinho `#071240`, azul `#0C72B0`, cinza `#ACACAC`.
- Pendente (a confirmar com a clínica): arquivo original do logo em SVG ou PNG transparente
  (docs/05; docs/09 item 6).

## 2. Tokens

| Token | Valor | Uso |
|---|---|---|
| `--bg` | `#F2F3F6` | Fundo da página |
| `--card` | `#FFFFFF` | Cartões |
| `--line`, `--line2` | `#E8EAEF`, `#D9DDE5` | Divisórias e bordas de campo |
| `--ink`, `--ink2` | `#0F172A`, `#334155` | Texto |
| `--mute` | `#5B6474` | Texto secundário (contraste de pelo menos 4,5:1 no branco) |
| `--navy` | `#0A1A4A` | Ação principal, aba ativa, menu |
| `--blue`, `--blue2` | `#0C72B0`, `#3E9BDB` | Links, destaque, gráficos |
| Pastéis | rose, amber, green, violet, teal (cor e fundo `-bg`) | Ícones e status: fundo claro com texto escuro |
| Raios | 22, 16 e 12 px (`--r`, `--r2`, `--r3`) | Cartão, bloco, campo |

Regra: não criar cor nova solta na tela. Cor nova entra como token no design system, com contraste
verificado, e passa pela revisão.

## 3. Tipografia

- Família Inter.
- Números grandes: 36 px, peso 650, tracking de menos 0,03em, algarismos tabulares (`num`).
- Rótulos: 12 px, caixa alta, tracking 0,06em, **só em cabeçalho de tabela** (docs/05 proíbe
  maiúsculas espaçadas fora disso).
- Título de tela no modelo atual: 24 px, peso 650 (`Pacientes-Lista.dc.html`).

## 4. Componentes do mockup

| Classe | Uso |
|---|---|
| `card`, `card-h` | Cartão e cabeçalho de cartão |
| `tile t-*` | Ícone em quadrado pastel |
| `lbl` com `big` | Número em destaque com rótulo |
| `chip c-*` | Situação ou status; no máximo um por linha de lista |
| `seg` | Abas em pílula, ativa em marinho |
| `tabs` | Abas de seção da tela |
| `btn btn-p`, `btn-b` | Botão principal marinho; botão azul |
| `field` com `in` | Campo de formulário |
| `search` com `kbd` | Busca com atalho |
| `av` | Avatar com iniciais |
| `tb` | Tabela: ocupa 100% e quebra texto nas células; `nw` só em valores curtos |
| `li` | Linha de lista |
| `bar`, `bars`, `bv`, `tip` | Progresso e gráfico de barras com dica |
| `dimg` | Imagem DICOM de exemplo |
| `step` | Etapa ok, todo, warn ou bad |
| `slot-free` | Horário livre, sempre visível e clicável |
| `anat`, `hs`, `hs-lb` | Visualizador anatômico escuro com pontos e rótulos |
| `phone`, `tabbar` | Telas de app |
| `sr` | Texto só para leitor de tela |

Layout: `.app` (menu de 280 px mais conteúdo); `.split` com `.main`, `.aside`, `.half`; grades
`.g2`, `.g3`, `.g4` com largura mínima; painéis `.fill`, `.card.pane` e `.scroll` para a área de
trabalho que cabe na janela.

## 5. Menu lateral

- Componente único `design/mockups/Sidebar.dc.html`, importado por todas as telas com o item ativo
  por parâmetro: `<dc-import name="Sidebar" active="pacientes">`.
- Recolhível: completo (280 px) ou só ícones (88 px). Recolhido, cada ícone tem dica com nome e
  contador; alertas continuam como selo; grupos viram separadores. Preferência salva por usuário.
  Botão com `aria-expanded` e rótulo; foco visível; sem animação com movimento reduzido.
- Rola por dentro; só o grupo da tela atual vem aberto.
- **Cores por módulo:** cada módulo tem ícone em quadrado colorido (`.ni` com `k-<cor>`), sempre a
  mesma cor em todo o sistema (agenda azul, recepção verde-água, laudos violeta, faturamento laranja).
  Item ativo com cor cheia e ícone branco. Cor nunca é a única pista: o nome sempre aparece.
  Paleta disponível em `design/vc10.css`: navy, blue, teal, violet, green, amber, rose, indigo,
  pink, orange, gray.

## 6. Barra do paciente em foco

Mockup `Barra-Paciente.dc.html`, fixa no topo de toda tela clínica (sala, atendimento, laudo,
visualizador), em uma linha (DEC-2026-11-08-02):
- À esquerda: nome e **nascimento em destaque** (os dois identificadores nunca são cortados).
- No meio, com corte e dica: alergia em vermelho cheio (nunca cortada), pulseira conferida,
  alertas, exame e local.
- À direita: ações. "Trocar paciente" lista só os pacientes do dia daquela sala e salva o rascunho
  antes de trocar.

## 7. Chat da equipe

Janela flutuante (`Chat-Popout`) por cima de qualquer tela; Esc fecha; botão leva à página inteira
(`Chat-Interno`). Assunto de paciente ligado ao paciente e visível só com vínculo de cuidado;
urgente toca som e pede confirmação de leitura; edição mostra "editada" e guarda a anterior; sem
resultado ou diagnóstico em grupo geral.

## 8. Regras de UX aprendidas com o cliente

1. Nada espremido: nenhum bloco abaixo de cerca de 340 px; nomes em uma linha; células com até 2 linhas.
2. Uma próxima ação por cartão; o resto em "mais opções".
3. A pendência aparece onde se resolve ("Falta pedido" com botão Anexar).
4. Horários livres sempre visíveis e clicáveis.
5. Status com cor e texto (e símbolo SVG); azul e laranja em vez de verde e vermelho quando a diferença importa.
6. Realismo onde é clínico: anatomia real, imagens de exame reais (exemplos DICOM), nada de boneco.
7. Sem emoji e sem gradiente decorativo; gradiente só no fundo do visualizador anatômico e no esqueleto de carregamento.

## 9. Como criar ou alterar um mockup

1. Copiar a estrutura de `design/mockups/Pacientes-Lista.dc.html`: `<x-dc>` com `<helmet>` (fonte
   Inter e estilos locais mínimos), `.app` com `.side` importando o `Sidebar`, `main.content` com
   cabeçalho de uma linha, abas e área de trabalho.
2. A folha de estilos é referenciada pelo endereço `/_blob/...` do canvas; a cópia local é
   `design/vc10.css`. Estilo novo e reutilizável vai para o design system, não para o `<helmet>`.
3. Listas e repetições com `sc-for` e condições com `sc-if`; dados de exemplo coerentes com
   `design/dados-ficticios.md` (data de referência 07/11/2026).
4. Registrar a posição e notas da tela em `design/mockups/canvas.json`; atualizar a tabela de
   telas do `docs/06-mockups.md` quando a tela for nova.
5. Os geradores `tools/mockups/gerar_*.py` estão aposentados (DEC-2026-11-08-05): não usar.
6. Campos pendentes da clínica entre colchetes: `[ENDEREÇO]`, `[CNPJ]`, `[CRM]`.
7. Textos clínicos e preparos são ilustrativos até revisão do responsável técnico.
8. Rodar as verificações de `references/padroes-de-tela.md` § 7.

## 10. Do mockup para o código

- O design system vira o pacote `packages/design-system/` (tokens e componentes), a partir do CSS
  do design system (docs/11 § 9.1). Componentes acessíveis do design system em vez de HTML
  improvisado (docs/14 B3, TypeScript).
- Componentes únicos previstos: `MenuDeContexto` (docs/19 § 8), barra do paciente, menu lateral,
  estados de tela.
- O nome do componente e do token no código segue o do mockup, para a revisão comparar lado a lado.
- Biblioteca de ícones SVG de traço: a confirmar na implementação do pacote, mantendo o traço e o
  tamanho usados nos mockups.
