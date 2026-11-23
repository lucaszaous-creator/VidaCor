# 31 · Visualizador VidaCor: abrir exames DICOM e exportar PDF e JPEG para o paciente

**Status:** proposta aguardando aprovação do dono do produto (G6). Nenhum código foi escrito.
**Decisão de origem:** DEC-2026-11-23-01 em [decisoes.md](decisoes.md).
**Base (ADR-0022):** prática de RIS/PACS (OHIF, visualizadores que acompanham o CD do exame, impressão
de imagens em papel no lugar do filme). Como a Medware e a Mediworks exportam imagens para o paciente:
**não verificado** (não há acesso aos sistemas nesta sessão; levantar com a equipe, item P-03).

## 1. O que é

Um programa para os computadores da clínica que:

1. **abre** exames DICOM (pasta, arquivo `.zip`, CD/DVD com `DICOMDIR` ou arquivos `.dcm` soltos);
2. **mostra** as imagens: séries, troca de corte, janela (partes moles, osso, cérebro, pulmão e ajuste
   livre), zoom e arrasto, dados do exame;
3. **exporta** para o paciente:
   - **PDF** com as imagens escolhidas e os dados do paciente;
   - **JPEG** das imagens escolhidas.

O sistema completo descrito nos documentos 01 a 30 **fica pausado, não apagado** (ADR-0019): o código e os
documentos continuam no repositório e o visualizador reaproveita o que serve (Typst em `crates/documentos`,
design system, regras de qualidade).

## 2. O que foi visto nas amostras

Cinco cortes de TC de crânio, anonimizados antes de qualquer uso no desenvolvimento:

| Item | Valor | Consequência para o software |
|---|---|---|
| Aparelho | Canon Aquilion Start | Primeiro aparelho a cobrir |
| Compressão | JPEG Lossless (1.2.840.10008.1.2.4.70) | Precisa de decodificador; o navegador sozinho não abre |
| Pixels | 512×512, 16 bits com sinal, MONOCHROME2 | Converter para 8 bits exige escolher a janela |
| Janela do aparelho | centro 35, largura 100 | Usar como padrão ao abrir |
| Texto gravado na imagem | não há | Os dados do paciente no PDF vêm do cabeçalho DICOM |
| Arquivos recebidos | **com dados reais do paciente** | Ver regra VIS-S5 sobre amostras |

## 3. Como será feito (recomendação)

| Parte | Escolha | Por quê |
|---|---|---|
| Programa | **Tauri** (Windows), sem servidor | Já é a pilha do projeto; roda sem internet e sem rede; instalador assinado |
| Leitura DICOM | `dicom-rs` em Rust (`dicom-object`, `dicom-pixeldata`) | Já previsto no docs/11; lê JPEG Lossless; a decodificação fica fora da tela |
| Tela | React + design system VidaCor | Mesma linguagem visual dos mockups |
| PDF | **Typst** (`crates/documentos`) com dados escapados | Já é o motor de documentos do projeto (docs/11, SEC de injeção) |
| JPEG | gerado em Rust com a janela que está na tela | O que o paciente recebe é o que foi visto |

Alternativa considerada: abrir o exame pelo **Orthanc/OHIF** que o projeto já prevê. Fica para uma fase 2,
quando os exames já estiverem no PACS. O pedido atual é abrir os arquivos que a clínica já tem.

## 4. Requisitos

| ID | Requisito |
|---|---|
| VIS-01 | Abrir pasta, `.zip`, `DICOMDIR` e `.dcm` soltos; arquivos que não forem DICOM são listados como ignorados, sem travar |
| VIS-02 | Agrupar por paciente, estudo e série; ordenar cortes pela posição (InstanceNumber como reserva) |
| VIS-03 | Ler as compressões usadas pelos aparelhos da clínica (primeiro: JPEG Lossless; confirmar as dos outros aparelhos) |
| VIS-04 | Janelas prontas (partes moles 40/400, cérebro 40/80, osso 500/2000, pulmão -600/1500, padrão do aparelho) e ajuste com o mouse |
| VIS-05 | Trocar de corte com a roda do mouse e o teclado; zoom e arrasto; voltar ao original |
| VIS-06 | Barra do paciente sempre visível com **nome e data de nascimento** (dois identificadores), exame e data |
| VIS-07 | Marcar imagens para exportar; a seleção mostra quantas e de qual série |
| VIS-08 | PDF de **um único estudo e um único paciente**, com 1, 2×2, 3×2 ou 4×3 imagens por página |
| VIS-09 | Cabeçalho do PDF: logotipo, nome e nascimento do paciente, exame, data do exame, nº de atendimento, médico solicitante, série e janela de cada imagem, página "x de y" |
| VIS-10 | Rodapé do PDF: "Imagens para referência. O documento oficial do exame é o laudo assinado." e data/hora da geração |
| VIS-11 | JPEG por imagem, nome do arquivo sem dado pessoal (ex.: `exame-2026-0001-serie2-img035.jpg`) |
| VIS-12 | Registro de cada exportação: quem, quando, estação, estudo (UID), quantas imagens, hash do arquivo gerado, sem nome do paciente no registro |
| VIS-13 | Tela cabe em 1440×900 e 1280×800 sem rolar a página; imagem e lista de cortes rolam dentro do painel (docs/05; o visualizador é exceção só para rolagem lateral) |
| VIS-14 | Menu de contexto na imagem e na série (docs/19): exportar esta imagem, marcar, copiar dados técnicos |

## 5. Segurança e proteção do paciente

| ID | Regra |
|---|---|
| VIS-S1 | **Um PDF, um paciente.** Se a seleção tiver imagens com PatientID, nome ou StudyInstanceUID diferentes, a exportação é **bloqueada** e a tela diz quais imagens divergem |
| VIS-S2 | Antes de gerar o PDF, a tela mostra nome e nascimento do paciente e pede confirmação (ato que sai da clínica, R2) |
| VIS-S3 | Nenhum dado do paciente em log, nome de arquivo temporário ou relatório de erro; temporários apagados ao fechar |
| VIS-S4 | Login no programa (usuário da clínica); exportar exige o usuário logado (SEC de reautenticação para exportação) |
| VIS-S5 | Desenvolvimento e testes **só com exames anonimizados ou públicos**; os arquivos reais recebidos nesta sessão não entram no repositório (G4) |
| VIS-S6 | Arquivo malformado ou gigante não trava nem derruba: limites de tamanho, de quantidade de arquivos no `.zip` e de descompressão (SEC de disponibilidade) |
| VIS-S7 | PDF não é diagnóstico: o JPEG perde tons; o aviso do VIS-10 é obrigatório |

Riscos propostos para o responsável clínico (entram no registro só com o aceite):

| ID | Risco | Controle |
|---|---|---|
| RCP-VIS-01 | Paciente recebe imagens de outra pessoa | VIS-S1, VIS-S2, VIS-06 |
| RCP-VIS-02 | Imagem lida como diagnóstica sem laudo | VIS-10, VIS-S7 |
| RCP-VIS-03 | Dados do paciente vazam por arquivo temporário ou log | VIS-S3, VIS-12 |

## 6. Frases de comportamento propostas (aguardam aprovação, G4)

- **FC-VIS-01** Quando o usuário abre o `.zip` de uma TC, então as séries aparecem agrupadas com nome e
  nascimento do paciente na barra.
- **FC-VIS-02** Quando o usuário marca 12 imagens de um mesmo estudo e pede o PDF em 3×2, então sai um
  PDF de 2 páginas com nome, nascimento, exame e data em todas as páginas.
- **FC-VIS-03** Quando a seleção tem imagens de dois pacientes, então o PDF não é gerado e a tela mostra
  quais imagens são de outro paciente.
- **FC-VIS-04** Quando o `.zip` tem arquivos que não são DICOM, então eles são listados como ignorados e
  o resto abre normalmente.
- **FC-VIS-05** Quando o usuário exporta JPEG na janela de osso, então a imagem salva é igual à mostrada
  na tela.

## 7. Etapas

| Etapa | Entrega | Aceite |
|---|---|---|
| E0 | Este plano aprovado; frases aprovadas; amostras anonimizadas de cada aparelho | Dono do produto |
| E1 | Núcleo Rust: ler, agrupar, decodificar, aplicar janela, gerar JPEG; testes com as amostras | Testes passando |
| E2 | Tela: abrir, navegar, janelas, zoom, barra do paciente | Teste com a equipe da recepção |
| E3 | PDF Typst com VIS-08 a VIS-10 e bloqueio VIS-S1 | Modelo aprovado pelo dono e pelo responsável clínico |
| E4 | Login, registro de exportação, instalador Windows assinado | Instalação em uma estação piloto |

## 8. Pendências (a confirmar com o dono do produto)

| ID | Pergunta | Premissa usada até a resposta |
|---|---|---|
| P-01 | Quais outros aparelhos e modalidades (ultrassom, raio-X, ressonância, mamografia)? | Só TC Canon na E1 |
| P-02 | De onde vêm os arquivos: CD, pasta na rede, exportação do PACS atual? | `.zip` e pasta local |
| P-03 | Como a Medware/Mediworks entrega imagens hoje (papel, CD, portal)? | Não verificado |
| P-04 | Quem gera o PDF: recepção, técnico ou médico? | Recepção e técnico |
| P-05 | Os computadores são Windows? Qual versão? | Windows 10/11 |
| P-06 | Logotipo e dados da clínica para o cabeçalho | Logotipo em `design/` |
| P-07 | O PDF vai impresso, por e-mail/WhatsApp ou pelo portal? | Arquivo salvo e impresso |
