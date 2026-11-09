// Modelo comprovante-preparo-1 (Preparo-Exame.dc.html): comprovante do agendamento com o preparo.
// SEC-INJ-4: todo dado vem de /dados.json e entra no documento como texto, nunca como marcação.

#let d = json("/dados.json")
#let azul = rgb("#0A1A4A")
#let cinza = rgb("#4B5563")
#let linha = rgb("#D9DDE5")
#let mono = "DejaVu Sans Mono"

#set document(title: "Comprovante " + d.emissao.codigo, author: d.clinica.nome)
#set text(font: "Libertinus Serif", size: 11pt, lang: "pt", region: "BR", fill: rgb("#111827"))
#set par(justify: false, leading: 0.6em)
#set page(
  paper: "a4",
  margin: (top: 16mm, bottom: 18mm, x: 18mm),
  footer: context {
    set text(size: 8pt, fill: cinza)
    line(length: 100%, stroke: 0.5pt + linha)
    [Comprovante #d.emissao.codigo do agendamento #d.agendamento, emitido em #d.emissao.em por #d.emissao.por. Modelo #d.emissao.modelo.]
  },
)

#let rotulo(t) = text(size: 8pt, weight: "semibold", fill: cinza, tracking: 0.04em, upper(t))
#let campo(r, valor) = [#rotulo(r) \ #valor]
#let secao(t) = block(above: 1.2em, below: 0.6em, text(size: 13pt, weight: "bold", fill: azul, t))

#text(size: 15pt, weight: "bold", fill: azul, d.clinica.nome)
#set text(size: 9pt, fill: cinza)
#for l in d.clinica.linhas [ \ #l]
#set text(size: 11pt, fill: rgb("#111827"))
#v(4pt)
#line(length: 100%, stroke: 1pt + azul)

#secao("Seu exame está marcado")
#grid(
  columns: (auto, 1fr),
  column-gutter: 18pt,
  block(inset: 10pt, stroke: 1pt + azul, radius: 6pt)[
    #align(center)[
      #text(size: 10pt, fill: cinza, d.dia_semana) \
      #text(size: 20pt, weight: "bold", fill: azul, d.data) \
      #text(size: 16pt, weight: "bold", d.hora)
    ]
  ],
  grid(
    columns: (1fr, 1fr),
    column-gutter: 14pt,
    row-gutter: 10pt,
    campo("Exame", text(weight: "bold", d.exame)),
    campo("Paciente", [#text(weight: "bold", d.paciente.nome) \ Nascido(a) em #d.paciente.nascimento]),
    campo("Convênio", d.convenio),
    campo("Chegue às", [#text(weight: "bold", d.chegada) (#d.antecedencia)]),
    campo("Duração", d.duracao),
    campo("Check-in", [Diga o seu nome e a data de nascimento na recepção.]),
  ),
)

#secao("Como se preparar")
#if d.passos.len() == 0 [
  Não há orientação de preparo cadastrada para este exame. Se tiver dúvida, fale com a recepção antes do dia.
] else [
  #for (i, p) in d.passos.enumerate() {
    block(below: 0.5em, grid(columns: (16pt, 1fr), text(weight: "bold", fill: azul, str(i + 1) + "."), p))
  }
  #text(size: 8.5pt, fill: cinza)[Orientação versão #d.preparo_versao, vigente desde #d.preparo_desde.]
]

#secao("Seu resultado")
O laudo fica disponível no portal do paciente depois de assinado pelo médico.

#secao("Precisa remarcar?")
Fale com a recepção com pelo menos 24 horas de antecedência, informando o código #text(font: mono, size: 10pt, d.agendamento).
