# 31 · Visualizador VidaCor: abrir exames DICOM e exportar PDF e JPEG para o paciente

**Status:** plano aprovado pelo dono do produto em 23/11/2026 (DEC-2026-11-23-02). E1 (núcleo), E2 (tela) e
E3 (PDF) entregues; E4 (login, instalador assinado, piloto) a fazer. Riscos RCP-VIS seguem aguardando o
responsável clínico; teste de usabilidade com a equipe ainda não feito.
**Decisões:** DEC-2026-11-23-01 a DEC-2026-11-23-14 em [decisoes.md](decisoes.md).
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

### 7.1 E1 entregue (núcleo, `crates/visualizador`)

- Abre pasta (com subpastas e `.zip` dentro), `.zip` e arquivo solto; nada é extraído para o disco;
  imagem repetida (mesmo SOPInstanceUID, comum em CD) entra uma vez só.
- Agrupa por paciente + estudo (o mesmo UID de estudo com paciente diferente fica separado) e série;
  ordena os cortes pela posição ao longo da normal, depois pelo InstanceNumber.
- Lê: sem compressão (implícito e explícito), JPEG Baseline, JPEG Lossless, JPEG 2000, JPEG-LS, RLE e
  Deflate; MONOCHROME1 e 2, RGB, YBR_FULL, YBR_FULL_422 e PALETTE COLOR; vários quadros (cine).
- Janela do arquivo (todas as gravadas), automática e as quatro prontas da TC; reescala aplicada.
- JPEG com a mesma renderização da tela; nome do arquivo sem dado pessoal.
- Limites de quantidade, tamanho de arquivo, tamanho total e tamanho de imagem.
- 29 testes (`cargo test -p vidacor-visualizador`) com exames **sintéticos** gerados por
  `tools/imagens-dicom/gerar_amostras_teste.py` (TC, RX, MG, RM, US). Conferência extra feita só na
  sessão, fora do repositório, com 5 cortes reais anonimizados do Canon Aquilion Start: abriram e a
  imagem saiu igual à conversão de referência.
- Ferramenta de conferência: `cargo run -p vidacor-visualizador --example exportar_jpeg -- <exame> <saída>`.

**Limites conhecidos da E1 (não verificados com aparelho real):**
- VOI LUT em tabela (alguns RX digitais) e função SIGMOID (algumas mamografias): hoje usa a janela
  numérica ou a automática; conferir quando houver amostra desses aparelhos.
- Paleta segmentada, cor em 16 bits, JPEG-XL e HTJ2K: o arquivo abre no índice, mas a imagem mostra
  "formato que o visualizador ainda não lê".
- Arquivo DICOM sem o cabeçalho de arquivo (ACR-NEMA antigo): listado como "não é DICOM".
- Compilar no Windows exige as ferramentas de C/C++ do Visual Studio e o CMake (OpenJPEG e CharLS); fica para a E4.

### 7.2 E2 e E3 entregues (tela e PDF)

**Onde:** `apps/visualizador` (tela React) e `apps/visualizador/src-tauri` (programa Tauri, Rust); PDF em
`crates/documentos/src/imagens.rs` com o modelo `crates/documentos/modelos/imagens.typ` (versão `imagens-exame-1`).

**Tela (1440×900 e 1280×800, sem rolar a página):**
- topo com logotipo, "Abrir pasta" e "Abrir arquivo"; barra do paciente com nome e nascimento (VIS-06);
- à esquerda, exames e séries (com aviso quando a abertura tem mais de um paciente) e os arquivos que
  ficaram de fora, com o motivo;
- no meio, a imagem: janelas do aparelho e prontas (teclas 1 a 4), arrastar ajusta contraste ou move,
  roda troca o corte, Ctrl + roda amplia, setas, PageUp/PageDown, Home/End, M marca, R volta à janela do
  aparelho; menu de contexto por clique direito, Shift+F10, tecla Menu e botão de mais opções (VIS-14);
- à direita, as imagens marcadas (cada uma com a janela com que foi marcada), imagens por página
  (1, 4, 6 ou 12), "Gerar PDF do paciente", "Salvar JPEG" e "Desmarcar todas". Com exames diferentes
  marcados o botão do PDF fica apagado com o motivo (R1); o Rust confere de novo (VIS-S1).
- antes do PDF, confirmação com nome e nascimento (VIS-S2); depois, aviso de onde foi salvo.

**Regras no Rust (a tela só pede):** o diálogo de arquivo é aberto pelo Rust (a tela nunca manda caminho);
os dados impressos vêm do exame; os pixels chegam à tela sem perda, com a mesma renderização do JPEG;
arquivo existente nunca é sobrescrito (`-2`, `-3`); cada exportação vai para o registro encadeado (VIS-12)
em `%APPDATA%\br.com.vidacorimagem.visualizador\registro-exportacoes.jsonl`, sem nome do paciente.

**PDF:** A4 retrato, PDF/A-2b; cabeçalho em todas as páginas com logotipo, nome e nascimento em destaque,
data do exame, exame, atendimento e solicitante; imagens em fundo preto com legenda (série, imagem, quadro,
janela); rodapé com o aviso do laudo, data da geração e página "x de y" (VIS-08 a VIS-10).

**Testes:** 15 do PDF em `crates/documentos`, 8 do programa em `apps/visualizador/src-tauri/tests`
(PDF, dois pacientes, dois exames, índices fora do exame, legenda, JPEG, nome livre, registro e
adulteração), 9 da tela com a ponte simulada (`pnpm --filter @vidacor/visualizador test`) e a conferência
"cabe na janela" no Chromium (`test:telas`). O programa Tauri foi aberto de verdade em tela virtual
(WebKitGTK); o fluxo completo pelo diálogo de arquivo **não foi verificado** de forma automática (o diálogo
nativo não é acionável no teste) e fica para a conferência manual no Windows (E4).

**Como rodar em desenvolvimento:** `just visualizador`. **Instalador:** `just visualizador-instalador`
no Windows (gera `.msi` e `.exe`); precisa das ferramentas de C/C++ do Visual Studio, CMake e WebView2.

## 8. Pendências

| ID | Pergunta | Resposta ou premissa |
|---|---|---|
| P-01 | Quais aparelhos e modalidades? | Dono do produto não sabe; pediu **todos que usam DICOM**. Atendido pela lista de formatos da E1; amostras anonimizadas de cada aparelho continuam bem-vindas |
| P-02 | De onde vêm os arquivos: CD, pasta na rede, exportação do PACS atual? | Premissa: `.zip`, pasta local e CD |
| P-03 | Como a Medware/Mediworks entrega imagens hoje (papel, CD, portal)? | Não verificado (sem acesso aos sistemas) |
| P-04 | Quem gera o PDF? | **Técnico, recepção e médico** (respondido em 23/11/2026) |
| P-05 | Os computadores são Windows? | **Sim, Windows** (respondido em 23/11/2026); versão a confirmar, premissa Windows 10/11 |
| P-06 | Logotipo e dados da clínica para o cabeçalho | Logotipo de `design/assets/marca/logo.png` (recorte; original vetorial ainda a pedir, docs/10). **Respondido em 23/11/2026:** VIDACOR SERVICOS MEDICOS MACAE LTDA, CNPJ 07.000.379/0001-89, Av. Nossa Senhora da Glória, 1225, Praia Campista, Macaé, RJ. **Telefone e responsável técnico: a confirmar** |
| P-08 | Quem usa: login próprio do programa ou o login do Windows? | Premissa até a E4: registro com o usuário do Windows; login próprio (VIS-S4) na E4 |
| P-07 | Como o PDF chega ao paciente? | **Arquivo**; a clínica escolhe enviar ou imprimir (respondido em 23/11/2026) |

## 9. Instalação e liberação (E4)

**Como o instalador é feito:** a CI (`.github/workflows/visualizador-windows.yml`) roda em Windows, testa a
tela e o Rust no próprio Windows e gera `Visualizador VidaCor_0.1.0_x64-setup.exe` (NSIS, instala para
todos os usuários da máquina, pede administrador) e o `.msi` (para instalação pela TI em várias máquinas),
com o SHA-256 de cada um. O artefato fica 30 dias na execução da CI. O WebView2 (já presente no
Windows 11 e no Windows 10 atualizado) é instalado pelo próprio instalador se faltar, o que pede internet
nessa hora.

**Antes de usar com paciente (portões do docs/16), cada um com quem aprova:**

| # | Item | Quem | Situação |
|---|---|---|---|
| L1 | Instalador gerado e testado no Windows pela CI | Claude | Feito na CI (ver a execução) |
| L2 | Instalação numa estação piloto e roteiro de conferência manual (abrir pasta, CD e `.zip`; gerar PDF e JPEG; conferir nome e nascimento; tentar dois pacientes) | TI da clínica com o técnico | A fazer |
| L3 | Parecer do responsável clínico de segurança sobre RCP-VIS-01 a 03 e o modelo do PDF | Responsável clínico | A fazer (bloqueia uso com paciente) |
| L4 | Teste de usabilidade com técnico, recepção e médico nas tarefas "abrir exame" e "gerar PDF" (docs/26) | Equipe da clínica | A fazer |
| L5 | Assinatura de código do instalador (certificado de assinatura em nome da empresa); sem ela o Windows mostra o aviso "editor desconhecido" | Dono do produto (compra do certificado) | A decidir; piloto pode usar sem assinatura, com o SHA-256 conferido pela TI |
| L6 | Login: usuário do Windows (hoje) ou login próprio do programa (VIS-S4, P-08) | Dono do produto | A decidir |
| L7 | Telefone e responsável técnico no cabeçalho do PDF (P-06) | Dono do produto | A informar (o PDF já sai sem eles, sem inventar) |
| L8 | Liberação: piloto em uma estação por pelo menos uma semana, depois as demais; sem liberar em véspera de feriado | Dono do produto | A fazer |
