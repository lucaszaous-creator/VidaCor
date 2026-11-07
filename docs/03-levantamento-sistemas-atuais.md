# 03 · Levantamento dos sistemas que a clínica usa hoje

Nomes confirmados pelo time: **Medware**, **Mediworks** ("Medworks") e
**Medilab** ("Medlab"), além de um conversor de imagem para DICOM. São dois
fornecedores brasileiros, com funções que se sobrepõem.

## Medware (medware.com.br)

Mais de 6 mil clínicas, forte em ecocardiograma e ultrassom.

**Clínicas Basic / Premium / Metrics** (gestão)
- Agenda e recepção: telão, totens, etiquetas, orçamentos, autorizações, status
  personalizáveis, atendimento por senha
- Prontuário e imagem: Laudos Flex, referências médicas, DICOM, captura de imagem,
  gestão de exames, bulário e Memed
- Faturamento: guias, lotes, transmissão, glosas
- Financeiro: fluxo de caixa, plano de contas, DRE, nota fiscal, **pagamento de
  médicos**
- Estoque, cadastros, backup automático, relatórios
- Metrics: painéis de produção, agenda, glosas e financeiro

**Laudos Flex / Flex+ / UX:** banco de frases, comparador de laudos, assinatura
digital; o UX roda em nuvem e transcreve ditado com IA.

**comDICOM:** armazenamento de imagens e cineloops, **worklist** (dados da agenda
vão para o aparelho), **DICOM SR** (medidas do aparelho entram no laudo) e DICOM
Print. É a provável ferramenta "que transforma imagem em DICOM".

**Extensões:** Meus Exames (web e app), **Meus Exames Bebê** (imagens e vídeos
obstétricos), App Agenda (médico), Paciente Conectado (confirmação por WhatsApp e
NPS), MultiChat (WhatsApp Business), Telemedicina, Web Agendamento, API Clínicas.

## Medilab Sistemas (medilab.net.br) — RIS/PACS

- **MediSystem PACS / Mediworks**: estação de trabalho do médico; todas as
  modalidades, comparação com exames anteriores, ditado por voz, priorização de
  laudos, IA (versão 11).
- **MediClinic RIS / Mediflow**: agenda de exames, controle de exame e laudo pelo
  navegador, impressão sob demanda (laudo, imagens, CD/DVD com a marca — **não será replicado**: a VidaCor não usa mais CD),
  agendamento online, MediAutonotify (SMS, e-mail, WhatsApp), **elegibilidade** no
  convênio.
- **MediWeb Portal**: entrega de exames e laudos a médicos e pacientes.
- **MediPrint DICOM**: conversão e impressão DICOM.

## Leitura do cenário

A clínica paga **dois sistemas que fazem coisas parecidas**: os dois têm agenda,
laudo e entrega de resultado. Provavelmente usa a Medware para gestão,
faturamento e laudo de eco/USG, e a Medilab para PACS e laudo de RX/TC.
**Unificar é a dor principal.**

## Comportamento a manter

- **Cadastro automático no portal**: a cada paciente novo, o sistema atual cria o
  acesso ao site, onde o paciente vê histórico, consultas, exames, laudos e
  documentos. O VidaCor mantém isso com segurança maior: link de uso único,
  senha criada pelo paciente e 2FA.

## Fontes

- https://www.medware.com.br/ · /clinicaspremium/ · /comdicom/
- https://medilab.net.br/ · /pacs/ · /solucoes/pacs-para-laudos/ · /solucoes/dicom-print/
- https://medilab.net.br/dia-mundial-da-saude-tecnologia-na-gestao-em-saude/ (Mediworks 11)
