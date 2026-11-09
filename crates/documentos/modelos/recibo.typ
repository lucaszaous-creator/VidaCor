// Modelo recibo-1 (Documentos.dc.html): recibo do recebimento pago no caixa.
// SEC-INJ-4: todo dado vem de /dados.json e entra no documento como texto, nunca como marcação.

#let d = json("/dados.json")
#let azul = rgb("#0A1A4A")
#let cinza = rgb("#4B5563")
#let linha = rgb("#D9DDE5")
#let mono = "DejaVu Sans Mono"

#set document(title: "Recibo " + d.emissao.codigo, author: d.clinica.nome)
#set text(font: "Libertinus Serif", size: 11pt, lang: "pt", region: "BR", fill: rgb("#111827"))
#set par(justify: true, leading: 0.65em)
#set page(
  paper: "a4",
  margin: (top: 18mm, bottom: 18mm, x: 20mm),
  footer: context {
    set text(size: 8pt, fill: cinza)
    line(length: 100%, stroke: 0.5pt + linha)
    [Recibo #d.emissao.codigo do recebimento #d.recebimento. O SHA-256 deste PDF fica registrado. Modelo #d.emissao.modelo.]
  },
)

#let rotulo(t) = text(size: 8pt, weight: "semibold", fill: cinza, tracking: 0.04em, upper(t))
#let campo(r, valor) = [#rotulo(r) \ #valor]

#grid(
  columns: (1fr, auto),
  [
    #text(size: 15pt, weight: "bold", fill: azul, d.clinica.nome)
    #set text(size: 9pt, fill: cinza)
    #for l in d.clinica.linhas [ \ #l]
  ],
  align(right + top)[
    #rotulo("Recibo") \
    #text(font: mono, size: 10pt, fill: azul, d.recebimento) \
    #text(size: 18pt, weight: "bold", fill: azul, d.valor)
  ],
)
#v(4pt)
#line(length: 100%, stroke: 1pt + azul)
#v(12pt)

Recebemos de #text(weight: "bold", d.pagador.nome)#if d.pagador.cpf != "" [, CPF #d.pagador.cpf], a quantia de #text(weight: "bold", d.valor) (#d.extenso), referente a:

#v(4pt)
#table(
  columns: (1fr, auto),
  inset: (x: 5pt, y: 5pt),
  stroke: (x, y) => if y == 0 { (bottom: 0.8pt + azul) } else { (bottom: 0.4pt + linha) },
  table.header(rotulo("Procedimento"), rotulo("Valor")),
  ..d.itens.map(i => (i.descricao, align(right, i.valor))).flatten(),
  ..if d.desconto != "" { ([Desconto], align(right, d.desconto)) } else { () },
  text(weight: "bold")[Total], align(right, text(weight: "bold", d.valor)),
)

#v(10pt)
#grid(
  columns: (1fr, 1fr, 1fr, 1fr),
  column-gutter: 12pt,
  campo("Data do pagamento", d.pago_em),
  campo("Forma", d.forma),
  campo("NFS-e", d.nfse),
  campo("Recebido por", d.recebido_por),
)
#v(30pt)
#align(center)[
  #line(length: 55%, stroke: 0.6pt + cinza)
  #d.clinica.nome \
  #text(size: 10pt, fill: cinza)[Emitido em #d.emissao.em por #d.emissao.por]
]
