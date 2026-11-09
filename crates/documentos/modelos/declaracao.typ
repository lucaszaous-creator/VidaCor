// Modelo declaracao-comparecimento-1 (Documentos.dc.html): declaração administrativa de comparecimento.
// SEC-INJ-4: todo dado vem de /dados.json e entra no documento como texto, nunca como marcação.

#let d = json("/dados.json")
#let azul = rgb("#0A1A4A")
#let cinza = rgb("#4B5563")
#let linha = rgb("#D9DDE5")
#let mono = "DejaVu Sans Mono"

#set document(title: "Declaração " + d.emissao.codigo, author: d.clinica.nome)
#set text(font: "Libertinus Serif", size: 12pt, lang: "pt", region: "BR", fill: rgb("#111827"))
#set par(justify: true, leading: 0.7em)
#set page(
  paper: "a4",
  margin: (top: 18mm, bottom: 18mm, x: 22mm),
  footer: context {
    set text(size: 8pt, fill: cinza)
    line(length: 100%, stroke: 0.5pt + linha)
    [Declaração #d.emissao.codigo. Para conferir a autenticidade, informe este código à clínica: o SHA-256 deste PDF fica registrado. Modelo #d.emissao.modelo.]
  },
)

#text(size: 15pt, weight: "bold", fill: azul, d.clinica.nome)
#set text(size: 9pt, fill: cinza)
#for l in d.clinica.linhas [ \ #l]
#set text(size: 12pt, fill: rgb("#111827"))
#v(4pt)
#line(length: 100%, stroke: 1pt + azul)
#v(18pt)
#align(center, text(size: 15pt, weight: "bold", fill: azul)[Declaração de comparecimento])
#v(14pt)

Declaramos que #text(weight: "bold", d.paciente.nome), nascido(a) em #d.paciente.nascimento#if d.paciente.cpf != "" [, CPF #d.paciente.cpf], compareceu a esta clínica no dia #text(weight: "bold", d.dia), com chegada registrada às #text(weight: "bold", d.chegada), para realizar o exame de #d.exame.

#v(6pt)
#text(size: 10pt, fill: cinza)[Esta declaração é administrativa e não substitui atestado médico.]

#v(30pt)
#align(center)[
  #line(length: 60%, stroke: 0.6pt + cinza)
  #d.emissao.por \
  #text(size: 10pt, fill: cinza)[Emitida em #d.emissao.em]
]
