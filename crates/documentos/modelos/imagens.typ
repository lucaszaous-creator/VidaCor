// Modelo imagens-exame-2 (docs/31 VIS-08 a VIS-10, DEC-2026-11-23-16): todas as imagens do exame,
// uma série por página, no arranjo das folhas que a clínica entrega hoje (Medilab).
// SEC-INJ-4: todo dado vem de /dados.json e entra como texto. As figuras são lidas por caminhos
// fixos montados aqui a partir do índice ("/imagem/<n>.jpg" e "/logo.png"), nunca de um texto do dado.
// A divisão em páginas e a grade de cada página vêm prontas do Rust (testadas lá).

#let d = json("/dados.json")
#let azul = rgb("#0A1A4A")
#let cinza = rgb("#4B5563")
#let linha = rgb("#D9DDE5")

#let rotulo(t) = text(size: 6.5pt, fill: cinza, t)
#let campo(r, valor) = grid(columns: (17mm, 1fr), column-gutter: 2mm, rotulo(r), valor)

// Cabeçalho repetido em todas as páginas: logotipo, os dois identificadores do paciente (VIS-09) e a
// empresa responsável.
#let cabecalho = {
  grid(
    columns: (auto, 1fr, auto),
    column-gutter: 8pt,
    align(left + horizon, image("/logo.png", height: 13mm)),
    align(left + horizon, block(width: 100%, {
      set text(size: 8.5pt)
      grid(
        columns: (1.35fr, 1fr),
        column-gutter: 8pt,
        row-gutter: 3.2pt,
        campo("Paciente", text(weight: "bold", size: 9.5pt, d.paciente.nome)),
        campo("Nascimento", text(weight: "bold", size: 9.5pt, d.paciente.nascimento)),
        campo("Exame", d.exame.descricao),
        campo("Data", d.exame.data),
        campo("Solicitante", d.exame.solicitante),
        campo("Número", d.exame.atendimento),
      )
    })),
    align(right + horizon, {
      set text(size: 6.5pt, fill: cinza)
      set par(leading: 0.45em)
      text(size: 9pt, weight: "bold", fill: azul, d.clinica.nome)
      for l in d.clinica.linhas [ \ #l]
    }),
  )
  v(1pt)
  line(length: 100%, stroke: 0.8pt + azul)
}

#set document(title: "Imagens do exame", author: d.clinica.nome)
#set text(font: "Libertinus Serif", size: 8.5pt, lang: "pt", region: "BR", fill: rgb("#111827"))
#set page(
  paper: "a4",
  margin: (top: 30mm, bottom: 13mm, x: 10mm),
  header-ascent: 4mm,
  header: cabecalho,
  footer-descent: 3mm,
  footer: context {
    set text(size: 6.5pt, fill: cinza)
    grid(
      columns: (1fr, auto),
      [Imagens para referência. O documento oficial do exame é o laudo assinado. Gerado em #d.gerado_em.],
      align(right)[#counter(page).display() de #counter(page).final().first()],
    )
  },
)

// Altura útil: 297 mm - 30 mm - 13 mm = 254 mm; 6 mm para o título da série; cada linha da grade
// reserva 4 mm para a legenda e o espaço entre linhas.
#let util = 248mm

#for (n, p) in d.paginas.enumerate() {
  if n > 0 { pagebreak() }
  block(height: 4mm, spacing: 2mm, text(size: 7.5pt, weight: "bold", fill: azul, p.titulo))
  let altura = (util - p.linhas * 4mm) / p.linhas
  grid(
    columns: (1fr,) * p.colunas,
    column-gutter: 1.5mm,
    row-gutter: 1mm,
    ..p.imagens.map(img => block(breakable: false, width: 100%, spacing: 0pt)[
      #box(
        width: 100%,
        height: altura,
        fill: black,
        clip: true,
        align(center + horizon, image("/imagem/" + str(img.indice) + ".jpg", width: 100%, height: 100%, fit: "contain")),
      )
      #v(0.6mm, weak: true)
      #text(size: 5.5pt, fill: cinza, img.legenda)
    ]),
  )
}
