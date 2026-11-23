// Modelo imagens-exame-1 (docs/31 VIS-08 a VIS-10): imagens do exame para entregar ao paciente.
// SEC-INJ-4: todo dado vem de /dados.json e entra como texto. As figuras são lidas por caminhos
// fixos montados aqui a partir do índice ("/imagem/<n>.jpg" e "/logo.png"), nunca de um texto do dado.

#let d = json("/dados.json")
#let azul = rgb("#0A1A4A")
#let cinza = rgb("#4B5563")
#let linha = rgb("#D9DDE5")
#let mono = "DejaVu Sans Mono"

#let rotulo(t) = text(size: 7pt, weight: "semibold", fill: cinza, tracking: 0.04em, upper(t))
#let campo(r, valor) = [#rotulo(r) \ #valor]

// Cabeçalho repetido em todas as páginas: clínica e os dois identificadores do paciente (VIS-09).
#let cabecalho = {
  grid(
    columns: (auto, 1fr),
    column-gutter: 10pt,
    align(left + horizon, image("/logo.png", height: 15mm)),
    align(right + horizon)[
      #text(size: 12pt, weight: "bold", fill: azul, d.clinica.nome)
      #set text(size: 7.5pt, fill: cinza)
      #for l in d.clinica.linhas [ \ #l]
    ],
  )
  v(2pt)
  line(length: 100%, stroke: 1pt + azul)
  v(1pt)
  grid(
    columns: (2fr, 1fr, 1fr),
    column-gutter: 12pt,
    row-gutter: 6pt,
    campo("Paciente", text(weight: "bold", size: 10.5pt, d.paciente.nome)),
    campo("Nascimento", text(weight: "bold", size: 10.5pt, d.paciente.nascimento)),
    campo("Data do exame", d.exame.data),
    campo("Exame", d.exame.descricao),
    campo("Atendimento", text(font: mono, size: 8.5pt, d.exame.atendimento)),
    campo("Médico solicitante", d.exame.solicitante),
  )
  v(1pt)
  line(length: 100%, stroke: 0.5pt + linha)
}

#set document(title: "Imagens do exame", author: d.clinica.nome)
#set text(font: "Libertinus Serif", size: 9.5pt, lang: "pt", region: "BR", fill: rgb("#111827"))
#set page(
  paper: "a4",
  margin: (top: 52mm, bottom: 18mm, x: 14mm),
  header-ascent: 6mm,
  header: cabecalho,
  footer: context {
    set text(size: 7.5pt, fill: cinza)
    line(length: 100%, stroke: 0.5pt + linha)
    grid(
      columns: (1fr, auto),
      [Imagens para referência. O documento oficial do exame é o laudo assinado. \ Gerado em #d.gerado_em.],
      align(right)[Página #counter(page).display() de #counter(page).final().first()],
    )
  },
)

// Altura útil da página: 297 mm - 52 mm - 18 mm = 227 mm. Cada linha da grade reserva 11 mm para a
// legenda e o espaço entre linhas.
#let por_pagina = d.grade.colunas * d.grade.linhas
#let altura = (227mm - d.grade.linhas * 11mm) / d.grade.linhas
#let total = d.imagens.len()

#for (n, inicio) in range(0, total, step: por_pagina).enumerate() {
  if n > 0 { pagebreak() }
  let fim = calc.min(inicio + por_pagina, total)
  grid(
    columns: (1fr,) * d.grade.colunas,
    column-gutter: 4mm,
    row-gutter: 5mm,
    ..range(inicio, fim).map(i => block(breakable: false, width: 100%)[
      #box(
        width: 100%,
        height: altura,
        fill: black,
        clip: true,
        align(center + horizon, image("/imagem/" + str(i) + ".jpg", width: 100%, height: 100%, fit: "contain")),
      )
      #v(1mm, weak: true)
      #text(size: 7pt, fill: cinza, d.imagens.at(i).legenda)
    ]),
  )
}
