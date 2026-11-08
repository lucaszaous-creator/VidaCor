// Modelo dossie-lastro-1 (docs/21, Explorador de lastro).
// SEC-INJ-4: todo dado vem de /dados.json e entra no documento como texto, nunca como marcação.

#let d = json("/dados.json")
#let azul = rgb("#0A1A4A")
#let cinza = rgb("#4B5563")
#let linha = rgb("#D9DDE5")
#let alerta = rgb("#B42318")
#let mono = "DejaVu Sans Mono"

#set document(title: "Dossiê do lastro " + d.emissao.codigo, author: d.clinica.nome)
#set text(font: "Libertinus Serif", size: 10pt, lang: "pt", region: "BR", fill: rgb("#111827"))
#set par(justify: false, leading: 0.55em)
#set page(
  paper: "a4",
  margin: (top: 16mm, bottom: 20mm, x: 15mm),
  footer: context {
    set text(size: 7.5pt, fill: cinza)
    line(length: 100%, stroke: 0.5pt + linha)
    grid(
      columns: (1fr, auto),
      [Dossiê #d.emissao.codigo, modelo #d.emissao.modelo. O SHA-256 deste PDF fica registrado no lastro do atendimento.],
      [Página #counter(page).display() de #counter(page).final().first()],
    )
  },
)

#let rotulo(t) = text(size: 7.5pt, weight: "semibold", fill: cinza, tracking: 0.04em, upper(t))
#let campo(r, valor) = [#rotulo(r) \ #valor]
#let secao(t) = block(above: 1.3em, below: 0.6em, text(size: 11pt, weight: "bold", fill: azul, t))
#let tabela-estilo = (x, y) => if y == 0 { (bottom: 0.8pt + azul) } else { (bottom: 0.4pt + linha) }

#grid(
  columns: (1fr, auto),
  [
    #text(size: 14pt, weight: "bold", fill: azul, d.clinica.nome)
    #set text(size: 8.5pt, fill: cinza)
    #for l in d.clinica.linhas [ \ #l]
  ],
  align(right + top)[
    #rotulo("Dossiê do lastro") \
    #text(font: mono, size: 9pt, fill: azul, d.emissao.codigo)
  ],
)
#v(4pt)
#line(length: 100%, stroke: 1pt + azul)
#v(4pt)

#grid(
  columns: (1.5fr, 1fr, 1fr),
  column-gutter: 14pt,
  row-gutter: 9pt,
  campo("Paciente", text(weight: "bold", size: 11pt, d.paciente.nome)),
  campo("Nascimento", d.paciente.nascimento),
  campo("Atendimento", text(font: mono, size: 9pt, d.atendimento)),
  campo("Emitido por", d.emissao.por),
  campo("Emitido em", d.emissao.em),
  campo("Código buscado", text(font: mono, size: 9pt, d.emissao.buscado)),
)

#v(8pt)
#if d.integra {
  block(width: 100%, inset: 8pt, stroke: (left: 2pt + azul))[
    *Cadeia íntegra.* Os #d.total eventos conferem: o hash de cada um bate com o conteúdo e com o evento anterior.
  ]
} else {
  block(width: 100%, inset: 8pt, stroke: 1pt + alerta)[
    #text(weight: "bold", fill: alerta)[Cadeia com divergência.]
    #d.falhas de #d.total eventos não conferem (marcados "NÃO" na coluna Confere). Encaminhe à auditoria; nada foi corrigido por este documento.
  ]
}

#secao("Registros ligados")
#set text(size: 9pt)
#table(
  columns: (auto, auto, auto, 1fr),
  inset: (x: 5pt, y: 4pt),
  stroke: tabela-estilo,
  table.header(rotulo("Tipo"), rotulo("Código"), rotulo("Situação"), rotulo("Criado em")),
  ..d.nos.map(n => (n.tipo, text(font: mono, size: 8pt, n.codigo), n.situacao, n.criado_em)).flatten(),
)

#secao("Eventos imutáveis")
#table(
  columns: (auto, auto, 1fr, auto, auto),
  inset: (x: 4pt, y: 4pt),
  stroke: tabela-estilo,
  table.header(rotulo("Seq."), rotulo("Data e hora"), rotulo("Evento e registro"), rotulo("Hash SHA-256"), rotulo("Confere")),
  ..d.eventos.map(e => (
    e.sequencia,
    e.quando,
    [#e.evento \ #text(size: 7.5pt, fill: cinza, e.detalhe)],
    block(width: 140pt, text(font: mono, size: 6.5pt, e.hash)),
    if e.confere == "Sim" { e.confere } else { text(weight: "bold", fill: alerta, e.confere) },
  )).flatten(),
)

#secao("Resumo da cadeia")
#set text(size: 9pt)
SHA-256 dos hashes dos eventos acima, em ordem de sequência, um por linha:
#v(2pt)
#text(font: mono, size: 8pt, d.resumo)
