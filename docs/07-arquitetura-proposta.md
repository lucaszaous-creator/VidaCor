# 07 · Arquitetura proposta

> Proposta inicial para discutir com o time. Nada aqui está implementado ainda.

## Visão geral

```
                    ┌──────────────── Servidor local da clínica ─────────────────┐
 Recepção / Caixa   │  VidaCor (app)        PostgreSQL         Orthanc (PACS)    │
 Consultório  ──────┼─►  .NET 8  ◄────────►  (dados)  ◄──────►  DICOM, worklist  │◄── Aparelhos
 Sala / Laudo       │      │                                     ▲   (eco, USG,   │    (DICOM)
                    │      ├── OHIF (visualizador web) ──────────┘    RX, TC…)    │
                    │      ├── Leitura de placa (fast-alpr) ◄── câmeras RTSP     │◄── Câmeras
                    │      └── Fila de integrações (WhatsApp, PIX, NFS-e, Memed)  │
                    └──────────────────────────┬─────────────────────────────────┘
                                               │ HTTPS (saída)
                     Portal/App do paciente ◄──┴──► Backup criptografado externo
```

## Decisões candidatas

| Tema | Proposta | Por quê |
|---|---|---|
| Base do sistema | Reaproveitar a suíte **Clinica** (.NET 8, EF Core, PostgreSQL, Blazor Hybrid) | Agenda, prontuário, **TISS 4.01 completo**, financeiro, estoque, WhatsApp e ICP-Brasil já existem e rodam em produção na Sem Dor |
| Onde roda | **Servidor local** da clínica + backup externo | Pedido do cliente; continua funcionando sem internet |
| Imagens (PACS) | **Orthanc** | Open source maduro, recebe DICOM e serve DICOMweb |
| Visualizador | **OHIF Viewer** (MIT), embutido no app | Web, MPR, medidas |
| Worklist / conversão | **DCMTK** (`wlmscpfs`, `img2dcm`) e **pydicom** | Padrões DICOM, licenças BSD/MIT |
| Leitura de placa | **fast-alpr** (MIT) | Leve, CPU, modelo com placas do Brasil |
| Portal e app | Web responsiva publicada com acesso externo seguro (túnel/reverse proxy), app depois | O paciente acessa de fora da clínica |
| Integrações | Memed, NFS-e da prefeitura, WhatsApp Business API, PIX (PSP), elegibilidade TISS | Paridade com Medware/Medilab |

## Mapa do reaproveitamento (suíte Clinica → VidaCor)

| Já existe | Precisa adaptar | Novo |
|---|---|---|
| Agenda multiprofissional, recepção, cadastro, prontuário que não apaga, TISS e glosas, financeiro e caixa, estoque, CRM/WhatsApp, permissões, assinatura ICP-Brasil, auditoria | Agenda por **sala/equipamento**, regras de convênio para exames de imagem, documentos com a marca VidaCor | Worklist e aparelhos, sala de exame, laudo com DICOM SR, PACS/visualizador, fluxo de laudos e CD, portal e app do paciente, Meus Exames Bebê, estacionamento, TV de chamada |

## Riscos técnicos

1. **Integração com os aparelhos reais**: depende de marca, modelo, AE Title e
   suporte a worklist/SR. Precisa de inventário e testes no local.
2. **Migração** de pacientes, laudos e imagens da Medware e da Medilab: as imagens
   DICOM migram bem; laudos e cadastros dependem de exportação do fornecedor.
3. **Anvisa (RDC 657/2022)**: software usado para dar diagnóstico sobre a imagem
   pode ser dispositivo médico. Estratégia: o laudo diagnóstico usa um
   visualizador já registrado, ou o OHIF entra só depois de avaliação
   regulatória. Gestão, laudo (texto) e faturamento não são afetados.
4. Volume de disco: definir retenção e capacidade com o volume mensal de exames.
