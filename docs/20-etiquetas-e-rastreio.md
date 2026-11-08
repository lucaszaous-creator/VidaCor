# 20 · Etiquetas e rastreio (lastro)

> **Status:** regra obrigatória (v0.1, nov/2026). Mockups:
> `design/mockups/Etiquetas.dc.html` (painel e lastro) e
> `design/mockups/Etiquetas-Modelos.dc.html` (modelos em tamanho real).
> Liga-se a: dois identificadores e IHE ([docs/16](16-protecao-do-usuario.md)),
> G2/G3 ([docs/15](15-padrao-de-qualidade.md)), auditoria e LGPD ([docs/08](08-seguranca-lgpd-etica.md), [docs/18](18-perfis-permissoes-e-rls.md)).

## 1. Para que serve
A etiqueta é a **ponte física** entre o paciente, o exame e o que sai da clínica.
O lastro responde, para qualquer etiqueta: **quem imprimiu, onde, quando, para
qual paciente e exame, onde foi bipada, se foi reimpressa e por quê, e onde ela
terminou.**

## 2. Tipos de etiqueta
| Tipo | Tamanho de referência | Quando é impressa | Onde é bipada |
|---|---|---|---|
| **Pulseira do paciente** | 250 × 25 mm | Check-in | Na sala, antes do exame |
| **Exame / pedido** | 60 × 30 mm | Check-in | Faturamento e conferência da guia |
| **Seringa de contraste** | 45 × 20 mm | Sala (TC) | No aparelho, antes da injeção |
| **Aparelho MAPA/Holter** | 60 × 30 mm | Instalação | Na devolução do aparelho |
| **Envelope de exame** | 110 × 60 mm | Liberação do laudo | Na entrega ao paciente |

## 3. O que vai impresso
- **Sempre:** nome completo, data de nascimento, código único da etiqueta (**ETQ**) e número do exame (**ACC**), em texto, código de barras e QR.
- **Quando se aplica:** "P" de preferencial; aviso de **ALERGIA** sem dizer qual (o detalhe só no sistema); data de devolução (MAPA).
- **Por tipo (decisão DEC-2026-11-08-03):**
  - pulseira: só identificação, "P" e o aviso genérico de ALERGIA (pode haver vários exames no dia);
  - exame e pedido: acrescenta **código TUSS e nome curto do exame**, data, hora e sala, para conferir pedido, sala e guia;
  - seringa de contraste: produto, volume, lote e validade;
  - MAPA/Holter: número do aparelho e data de devolução;
  - envelope: sem exame (sai da clínica); o resultado só abre no portal com login.
- **Nunca:** diagnóstico, hipótese, resultado, condição clínica (ex.: "gestante", qual alergia), CPF completo, convênio na pulseira, link que abra dado sem login.
- O QR do envelope leva ao portal, que **exige login** do paciente.

## 4. Regras do lastro
1. **Código único** por etiqueta (`ETQ-AAAA-NNNNNN`), gerado por sequência no banco e protegido por índice único (G2).
2. **Eventos imutáveis**: impressa → colocada → bipada (sala/aparelho) → vinculada ao exame → envelope → entregue. Cada evento grava quem, onde, quando e de qual equipamento.
3. **Reimpressão pede motivo** (rasgou, ilegível, dado corrigido) e **invalida a etiqueta anterior**. Bipar uma etiqueta invalidada **bloqueia** o ato e avisa.
4. **Bipe na sala confere paciente e exame**: se a pulseira não pertence ao exame agendado para aquela sala, o sistema **bloqueia**, não envia a worklist ao aparelho e abre uma **divergência** para resolver (perfil IHE PIR).
5. **Correção de dado do paciente** (nome, nascimento) invalida as etiquetas impressas e pede reimpressão.
6. **Imprimir de novo a mesma ação não duplica** (G3): o clique duplo não gera duas etiquetas.
7. **Contraste**: a etiqueta da seringa liga o **lote** ao paciente; o rastreio por lote aparece no Estoque (recall).
8. **Entrega**: o envelope é bipado na entrega; ACC divergente bloqueia.
9. Todo evento entra na **auditoria**; o lastro é consultável por perfil (recepção vê o operacional; dado clínico segue o docs/18).

## 5. Impressoras
- Impressoras térmicas (ZPL/ESC-POS) cadastradas por local (guichê, sala) em Configurações; o sistema escolhe a impressora pelo local da estação.
- Status (online, sem ribbon, sem etiqueta) e fila visíveis no painel; impressora offline desvia para a próxima do mesmo tipo, com aviso.

## 6. Modelos
- Modelos versionados (Typst ou ZPL gerado pelo núcleo), com a marca VidaCor, revisados pela clínica.
- Mudança de modelo passa por PR e é testada por **comparação visual** (docs/14, B4).

## 7. Testes obrigatórios (G4)
- Imprimir duas vezes ao mesmo tempo gera uma etiqueta só.
- Etiqueta invalidada bipada → bloqueio.
- Pulseira do paciente A bipada na sala do paciente B → bloqueio, divergência aberta, worklist não enviada.
- Correção de nascimento → etiquetas antigas invalidadas.
- Usuário sem permissão não vê dado clínico no lastro.
