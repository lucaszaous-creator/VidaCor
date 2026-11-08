// Modelo laudo-impresso-1 (design/mockups/Laudo-Impresso.dc.html).
// SEC-INJ-4: todo dado vem de /dados.json e entra no documento como texto (cadeia), nunca como
// marcação. Não use eval, read de outro arquivo nem concatenação de dado com código aqui.

#let d = json("/dados.json")
#let azul = rgb("#0A1A4A")
#let cinza = rgb("#4B5563")
#let linha = rgb("#D9DDE5")
#let mono = "DejaVu Sans Mono"

#set document(title: "Laudo " + d.laudo.codigo, author: d.clinica.nome)
#set text(font: "Libertinus Serif", size: 10.5pt, lang: "pt", region: "BR", fill: rgb("#111827"))
#set par(justify: true, leading: 0.62em, spacing: 0.9em)
#set page(
  paper: "a4",
  margin: (top: 16mm, bottom: 22mm, x: 18mm),
  footer: context {
    set text(size: 8pt, fill: cinza)
    line(length: 100%, stroke: 0.5pt + linha)
    grid(
      columns: (1fr, auto),
      [Laudo a ser interpretado pelo médico assistente com a história clínica.],
      [Página #counter(page).display() de #counter(page).final().first(), versão #d.laudo.versao],
    )
  },
)

#let rotulo(t) = text(size: 7.5pt, weight: "semibold", fill: cinza, tracking: 0.04em, upper(t))
#let campo(r, valor) = [#rotulo(r) \ #valor]
#let secao(t) = block(above: 1.2em, below: 0.5em, text(size: 11pt, weight: "bold", fill: azul, t))

// Cabeçalho da clínica
#grid(
  columns: (1fr, auto),
  column-gutter: 12pt,
  [
    #text(size: 15pt, weight: "bold", fill: azul, d.clinica.nome)
    #set text(size: 8.5pt, fill: cinza)
    #for l in d.clinica.linhas [ \ #l]
  ],
  align(right + top)[
    #set text(size: 8.5pt, fill: cinza)
    #rotulo("Laudo") \
    #text(font: mono, size: 9pt, fill: azul, d.laudo.codigo)
  ],
)
#v(4pt)
#line(length: 100%, stroke: 1pt + azul)

// Paciente: dois identificadores (nome e nascimento) sempre juntos
#v(4pt)
#grid(
  columns: (1.6fr, 1fr, 1fr),
  column-gutter: 14pt,
  row-gutter: 9pt,
  campo("Paciente", text(weight: "bold", size: 11.5pt, d.paciente.nome)),
  campo("Nascimento", d.paciente.nascimento),
  campo("Exame em", d.atendimento.data),
  campo("Exame", d.exame),
  campo("Protocolo", text(font: mono, size: 9pt, d.atendimento.protocolo)),
  campo("Versão", d.laudo.versao),
)
#v(2pt)
#line(length: 100%, stroke: 0.5pt + linha)

#v(6pt)
#text(size: 14pt, weight: "bold", fill: azul, d.exame)

#if d.laudo.substituida {
  block(width: 100%, inset: 8pt, stroke: 1pt + rgb("#B42318"))[
    #text(weight: "bold", fill: rgb("#B42318"))[Versão substituída.]
    Esta versão foi substituída por uma retificação assinada. Peça à clínica a versão vigente.
  ]
}

#if d.retificacao != none {
  block(width: 100%, inset: 8pt, stroke: (left: 2pt + azul))[
    #text(weight: "bold")[Retificação.]
    Esta versão #d.laudo.versao substitui a versão #d.retificacao.anterior. Motivo: #d.retificacao.motivo
  ]
}

#secao("Análise")
#for p in d.paragrafos {
  par(p.join(linebreak()))
}

#if d.achados.len() > 0 {
  secao("Achados estruturados")
  set text(size: 9.5pt)
  table(
    columns: (auto, auto, auto, 1fr, auto),
    inset: (x: 5pt, y: 5pt),
    stroke: (x, y) => if y == 0 { (bottom: 0.8pt + azul) } else { (bottom: 0.5pt + linha) },
    table.header(
      rotulo("Nº"), rotulo("Classificação"), rotulo("Categoria"), rotulo("Localização"), rotulo("Maior diâmetro"),
    ),
    ..d.achados.map(a => (a.numero, a.classificacao, text(weight: "bold", a.categoria), a.localizacao, a.diametro)).flatten(),
  )
  for a in d.achados {
    block(above: 0.8em)[
      #text(weight: "bold")[Achado #a.numero.] #a.descricao \
      #text(weight: "bold")[Conclusão:] #a.conclusao
    ]
  }
}

// Assinatura e conferência
#v(14pt)
#block(width: 100%, breakable: false, inset: (top: 9pt), stroke: (top: 0.5pt + linha))[
  #text(weight: "bold", size: 11pt, d.medico.nome)#if d.medico.registro != none [, #d.medico.registro] else [, registro profissional não cadastrado]#if d.medico.rqe != none [, RQE #d.medico.rqe]
  \
  #text(size: 9.5pt)[Assinado digitalmente em #d.assinatura.em (horário de Brasília).]
  #if d.assinatura.certificado != none [
    \ #text(size: 8.5pt, fill: cinza)[Certificado: #d.assinatura.certificado]
  ]
  #v(4pt)
  #grid(
    columns: (auto, 1fr),
    column-gutter: 10pt,
    row-gutter: 5pt,
    rotulo("Código de verificação"),
    text(font: mono, weight: "bold", size: 10pt, d.assinatura.codigo_verificacao),
    rotulo("SHA-256 da versão"),
    text(font: mono, size: 7.5pt, d.assinatura.hash),
  )
]
