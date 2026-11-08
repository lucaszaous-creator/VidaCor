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

Folha de estilos dos mockups: `design/vc10.css`. Verificação: `node tools/verificacao/sem-rolagem-lateral.js <pasta-dos-mockups> design/vc10.css 1000`.

**Sem rolagem lateral (regra):** em telas de 1280 px ou mais (menu de 280 px + 1000 px de conteúdo), **nenhuma tela obriga a arrastar para o lado**. Tabelas cabem na largura: menos colunas (junte atributos em uma célula), texto quebrando, detalhe abaixo da tabela em vez de coluna lateral. Proibido `min-width` fixo em tabela e quadro. Exceções: telão (tamanho fixo de TV) e o visualizador de imagens. Conferido por teste automático que mede a largura de cada tela.

O **menu lateral** é um componente único (`design/mockups/Sidebar.dc.html`)
importado por todas as telas, com o item ativo por parâmetro.

**Menu lateral recolhível:** botão no topo do menu alterna entre completo (280 px) e só ícones (88 px). Recolhido, cada ícone tem dica com o nome e o contador, alertas continuam visíveis como selo, e os grupos viram separadores. A escolha fica salva por usuário e vale em todas as telas. Botão com `aria-expanded` e rótulo; foco visível; sem animação quando o sistema pede movimento reduzido.

**Cores do menu:** cada módulo tem um ícone em quadrado colorido (`.ni` + `k-<cor>`), sempre a mesma cor em todo o sistema (agenda azul, recepção verde-água, laudos violeta, faturamento laranja…). O item ativo fica com a cor cheia e o ícone branco. Cor nunca é a única pista: o nome sempre aparece (ou na dica, com o menu recolhido).

**Chat da equipe:** abre do menu como janela flutuante (`Chat-Popout`) por cima de qualquer tela, sem perder o que se estava fazendo; Esc fecha; botão leva à página inteira (`Chat-Interno`). Regras:
- assunto de paciente é **ligado ao paciente** (nome + nascimento) e só quem tem vínculo de cuidado vê (docs/18);
- mensagem urgente toca som, fica no topo e pede confirmação de leitura;
- nada é apagado: edição mostra "editada" e guarda a anterior (docs/21);
- sem resultado de exame ou diagnóstico em grupo geral; arquivos ficam no servidor da clínica;
- substitui WhatsApp pessoal para assuntos internos.

**Barra do paciente em foco** (`Barra-Paciente`): fixa no topo de toda tela clínica (sala, atendimento, laudo, visualizador): nome, **nascimento em destaque**, idade, sexo, convênio, alergia em vermelho cheio, alertas, exame e local, pulseira conferida. "Trocar paciente" lista só os pacientes do dia daquela sala e salva o rascunho antes de trocar.

**Estados de tela** (`Estados`): todo bloco de dados tem desenhados vazio, carregando (esqueleto), sem conexão, erro, sem permissão, salvamento automático e conflito, feito com desfazer, ato irreversível, campo com erro, busca sem resultado, avisos em três níveis e central de avisos.

**Protótipo de fluxo** (`Fluxo-Completo`): agendar convênio com guia → check-in com dois identificadores → sala com bipe da pulseira (com simulação de pulseira errada) → laudo com assinatura → portal, com o lastro sendo montado ao lado. Usado no teste de usabilidade com a equipe (docs/16).

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


## Regras de acabamento (obrigatórias, 07/11/2026)

### A tela cabe na janela
- Toda tela de sistema é desenhada para **1440×900** e precisa funcionar em **1280×800** **sem rolar a página**.
- O que é longo rola **dentro do próprio painel** (lista, tabela, conversa, linha do tempo), com cabeçalho fixo.
- Conteúdo secundário vai para **abas**, **painel de detalhe** ou **gaveta**; nunca empilhado para baixo.
- Estrutura padrão: cabeçalho de uma linha (título, busca, ação principal) → abas (se houver) → área de trabalho
  com 1 a 3 painéis (`.fill`, `.card.pane`, `.scroll`). Modelo: `design/mockups/Pacientes-Lista.dc.html`.
- O menu lateral rola por dentro; só o grupo da tela atual vem aberto.
- Exceções com tamanho próprio: apps (390×844), tablet (1180×820), telão (1280×720) e impressos A4.
- Verificação: `tools/verificacao/verifica.sh` renderiza com o motor do canvas nas duas resoluções e falha se a
  página rolar, se um painel cortar conteúdo ou se houver rolagem lateral.
  Motor: `tools/verificacao/motor-dc/support.js` (aberto e versionado, DEC-2026-11-08-39); detalhes em
  `tools/verificacao/README.md`.

### Proibido "cara de IA"
- Emoji e caracteres usados como ícone (✓ ✕ ▶ ● ▼ ★ ⚡ ✨ ‹ › → ← ⋯ •). Ícone é SVG de traço do conjunto do sistema.
- Ícones de raio, brilho, estrela mágica, sol ou varinha; enfeites de "IA".
- Gradientes decorativos (só o esqueleto de carregamento).
- Pílula para tudo: chip só para situação/status, no máximo um por linha de lista.
- Correntes de "·": no máximo um separador por linha; usar colunas, rótulos ou vírgula.
- Rótulos em MAIÚSCULAS espaçadas fora de cabeçalho de tabela; quadradinhos coloridos numerados como enfeite de título.
- Cards de "Regras", "Dicas", "Como funciona" e parágrafos explicando o óbvio; tom de marketing; rodapés de "mockup".
- Borda colorida à esquerda em card, sombras pesadas, subtítulo em toda caixa.
