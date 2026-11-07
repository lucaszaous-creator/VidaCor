# 06 · Mockups

**Ao vivo (canvas navegável):** https://claude.ai/artifact/NT2bm1UnVPJe9NGGs3HTii
— privado; compartilhe pelo menu *Share* para o cliente ou o time abrirem. As
telas de sistema são clicáveis no modo *Play* (o menu leva de uma tela à outra).

**Fonte:** [`design/mockups/`](../design/mockups/). Cada tela é um arquivo
`.dc.html` (HTML com marcação de componente). O `canvas.json` guarda a posição
das telas e as notas. Imagens e CSS são referenciados por endereço `/_blob/…`
do canvas; as cópias locais estão em `design/assets/` (veja
[10](10-assets-e-licencas.md)).

## As telas (rodada 3, aprovada)

| # | Arquivo | Tela |
|---|---|---|
| — | `Sidebar.dc.html` | Menu lateral compartilhado |
| 1 | `Main.dc.html` | Início (painel do dia) |
| 2b | `Agendamento-Guia.dc.html` | Novo agendamento: convênio e guia (autorização) com conferências |
| 2 | `Agenda.dc.html` | Agenda por sala e agendamento |
| 3 | `Recepcao.dc.html` | Recepção, senhas, **esteira do paciente** e check-in |
| 4 | `Caixa.dc.html` | Caixa do particular |
| 4e | `Lastro.dc.html` | Explorador de lastro: cadeia completa do atendimento e metadados |
| 4c | `Etiquetas.dc.html` | Etiquetas e rastreio (lastro, divergências, impressoras) |
| 4d | `Etiquetas-Modelos.dc.html` | Modelos de etiqueta em tamanho real |
| 5 | `Paciente.dc.html` | Paciente 360º + anatomia |
| 5b | `Agenda-Medico.dc.html` | Minha agenda (médico) + onde estão meus pacientes |
| 6 | `Atendimento.dc.html` | Atendimento médico |
| 7 | `Sala-Exame.dc.html` | Sala de exame |
| 7b | `Aparelhos.dc.html` | Aparelhos e integração DICOM (imagem chega ao vivo) |
| 8 | `Laudo.dc.html` | Laudar (estação do médico) |
| 9 | `Visualizador.dc.html` | Visualizador de imagens (PACS) |
| 10 | `Fluxo-Laudos.dc.html` | Fluxo de laudos, entrega e impressão |
| 11 | `Faturamento.dc.html` | Faturamento TISS |
| 12 | `Financeiro.dc.html` | Financeiro |
| 13 | `Estoque.dc.html` | Estoque |
| 14 | `Comunicacao.dc.html` | WhatsApp, automações, NPS e telemedicina |
| 15 | `Indicadores.dc.html` | Indicadores |
| 16 | `Configuracoes.dc.html` | Configurações e segurança |
| 17 | `Portal-Web.dc.html` | Portal do paciente (web) |
| 18 | `App-Paciente.dc.html` | App do paciente |
| 19 | `App-Bebe.dc.html` | Meus Exames Bebê |
| 20 | `App-Medico.dc.html` | App do médico |
| 21 | `Telao.dc.html` | TV de chamada |
| 22 | `Laudo-Impresso.dc.html` | Laudo A4 |
| 23 | `Preparo-Exame.dc.html` | Comprovante + preparo A4 |
| 24 | `Documentos.dc.html` | Atestado, recibo e etiquetas A4 |
| 25 | `Menus-Contexto.dc.html` | Clique direito: 8 situações com menus e submenus + regras |
| 26 | `Recepcao-Interativa.dc.html` | **Protótipo interativo**: esteira com clique direito, teclado, submenus, seleção múltipla, perfis e confirmação |

## Rodadas

| Rodada | Resultado | O que mudou |
|---|---|---|
| 1 | Reprovada no visual | 7 telas simples, totem; "UI amadora" |
| 2 | "Caminho perfeito" | Design system novo, 22 telas, 3D de manequim, DICOM reais |
| 3 | **"Perfeito!"** | Anatomia realista, Atendimento, Caixa, Recepção refeita, layout sem aperto, funções extras |

## Convenções do mockup

- Dados **fictícios** e consistentes entre telas (pacientes Sônia Maria Castro,
  Maria Helena Duarte, Antônio Carlos Ribeiro, Juliana Sampaio Rocha, Rafael
  Monteiro Lima, Lúcia Fernandes Prado; médicos Dr. Paulo Medeiros, Dra. Renata
  Alves, Dr. Marcos Freire).
- Textos clínicos e preparos são **ilustrativos** e precisam ser revisados pelo
  responsável técnico.
- Campos pendentes da clínica ficam entre colchetes: `[ENDEREÇO]`, `[CNPJ]`,
  `[CRM]`.

## Vídeo comercial do estacionamento

[`design/video/VidaCor-Estacionamento-Comercial.mp4`](../design/video/VidaCor-Estacionamento-Comercial.mp4)
(69 s, 1080p, narração em português): confirmação pelo WhatsApp com a placa,
vínculo da placa ao paciente, leitura na entrada, check-in que já mostra o
veículo, permanência contada, cortesia ou pagamento por PIX, saída com a cancela
abrindo sozinha e painel de gestão. Animação feita em HTML e renderizada quadro a
quadro ([`tools/video-estacionamento/`](../tools/video-estacionamento/)). A
narração foi gerada no ElevenLabs (voz "Isabela"). Valores e regras de cortesia
são ilustrativos.
