# 05 · Design system VidaCor

Fonte: [`design/assets/vidacor.css`](../design/assets/vidacor.css), usado por
todos os mockups. As referências visuais aprovadas pelo cliente estão em
[`design/referencias/`](../design/referencias/).

## Marca

- Logo: "V" azul-marinho com cinza, **VIDACOR** (VIDA marinho, COR azul),
  **IMAGEM** e o slogan *Tecnologia que cuida da sua vida*. Os recortes estão em
  `design/assets/marca/`. **Pendente:** pedir o arquivo original (SVG ou PNG
  transparente).
- Cores amostradas do logo: marinho `#071240`, azul `#0C72B0`, cinza `#ACACAC`.

## Tokens

| Token | Valor | Uso |
|---|---|---|
| `--bg` | `#F2F3F6` | Fundo da página |
| `--card` | `#FFFFFF` | Cartões |
| `--line` / `--line2` | `#E8EAEF` / `#D9DDE5` | Divisórias e bordas de campos |
| `--ink` / `--ink2` | `#0F172A` / `#334155` | Texto |
| `--mute` | `#5B6474` | Texto secundário (≥ 4,5:1 no branco) |
| `--navy` | `#0A1A4A` | Ação principal, aba ativa, menu |
| `--blue` / `--blue2` | `#0C72B0` / `#3E9BDB` | Links, destaque, gráficos |
| Pastéis | rose, amber, green, violet, teal, sky | Ícones e status (fundo claro + texto escuro) |
| Raios | 22 / 16 / 12 px | Cartão / bloco / campo |

**Tipografia:** Inter. Números grandes com 36 px, peso 650, tracking −0,03em,
algarismos tabulares. Rótulos com 12 px, caixa alta, tracking 0,06em.

## Componentes

`card` · `card-h` · `tile t-*` (ícone em quadrado pastel) · `lbl` + `big` (número
destaque) · `chip c-*` (status) · `seg` (abas em pílula, ativa em marinho) ·
`btn btn-p` (marinho) / `btn-b` (azul) · `field` + `in` · `search` + `kbd` ·
`av` (avatar com iniciais) · `tb` (tabela) · `li` (linha de lista) · `bar`
(progresso) · `bars/bv/tip` (gráfico de barras com tooltip) · `dimg` (imagem
DICOM) · `step` (etapa ok/todo/warn/bad) · `slot-free` (horário livre) · `anat`
(visualizador anatômico escuro com `hs` hotspots e `hs-lb` rótulos) · `phone`
e `tabbar` (app).

**Layout:** `.app` (menu de 280 px + conteúdo) · `.split > .main / .aside /
.half` (colunas que quebram) · `.g2/.g3/.g4` (grades com largura mínima) ·
`.tb` ocupa 100% e quebra o texto nas células; `.nw` só em valores curtos (data, hora, valor).

Folha de estilos dos mockups: `design/vc4.css`. Verificação: `node tools/verificacao/sem-rolagem-lateral.js <pasta-dos-mockups> design/vc4.css 1000`.

**Sem rolagem lateral (regra):** em telas de 1280 px ou mais (menu de 280 px + 1000 px de conteúdo), **nenhuma tela obriga a arrastar para o lado**. Tabelas cabem na largura: menos colunas (junte atributos em uma célula), texto quebrando, detalhe abaixo da tabela em vez de coluna lateral. Proibido `min-width` fixo em tabela e quadro. Exceções: telão (tamanho fixo de TV) e o visualizador de imagens. Conferido por teste automático que mede a largura de cada tela.

O **menu lateral** é um componente único (`design/mockups/Sidebar.dc.html`)
importado por todas as telas, com o item ativo por parâmetro.

## Regras de UX (aprendidas com o cliente)

1. **Nada espremido:** tabela larga nunca vai em coluna estreita; nenhum bloco
   abaixo de cerca de 340 px; nomes em uma linha e células com até 2 linhas.
2. **Uma próxima ação** por cartão; o resto fica em "mais opções".
3. **A pendência aparece onde se resolve** ("Falta pedido" → botão Anexar).
4. **Horários livres sempre visíveis e clicáveis.**
5. **Status = cor + texto** (e símbolo), nunca só a cor; azul/laranja em vez de
   verde/vermelho quando a diferença importa.
6. **Realismo** onde é clínico: anatomia real, imagens de exame reais (exemplos
   DICOM), nada de "boneco".
7. Sem emoji, sem enfeite em gradiente; o gradiente fica restrito ao fundo do
   visualizador anatômico.

## Acessibilidade

Contraste AA · alvo de toque ≥ 44 px · controles reais (`button`, `a`, `label`) ·
`aria-label` em botão só com ícone · teclado (Ctrl+K busca, F2 check-in, F9
ditado) · pensado para 1366×768 e zoom de 150% · portal e app com tamanho de
letra ajustável.

## Menu de contexto

Todo objeto tem menu de clique direito (também por teclado e pelo botão ⋯). Regras completas e catálogo por objeto em [19 · Menu de contexto](19-menu-de-contexto.md).
