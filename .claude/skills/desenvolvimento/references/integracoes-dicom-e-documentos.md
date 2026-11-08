# Integrações, DICOM e documentos

Fontes: `docs/11-arquitetura-e-infraestrutura.md` §2, §4, §6; `docs/14-regras-de-arquitetura-e-engenharia.md` A5, A7;
`docs/15-padrao-de-qualidade.md` G3; `docs/16-protecao-do-usuario.md` C3; `docs/17-seguranca-por-classe-de-ataque.md`
B1, B6, B9, B12; `docs/21-lastro-e-rastreabilidade.md` §2 e §3.5; ADR-0006, ADR-0007, ADR-0008.

## 1. Princípios comuns

- Adaptadores ficam em `crates/integracoes` e `crates/dicom-gateway`, **na borda**. O domínio fala por traits.
- Toda saída para provedor passa pelo **outbox** e por worker com retentativa exponencial, jitter, teto e fila de falhas.
- **Chave de idempotência UUID** gerada uma vez por operação de negócio e reaproveitada em toda retentativa
  (cobrança e estorno PIX, NFS-e, lote TISS, assinatura, WhatsApp). Nunca CPF, e-mail, telefone ou nome.
- Timeout, circuit breaker e pool próprio por provedor; falha visível em Configurações e como pendência (R7).
- Identificadores externos guardados como recebidos, com índice único quando o provedor garante (txid, protocolo ANS,
  código da NFS-e, StudyUID).
- Credenciais por serviço, no cofre (SOPS/age), com escopo mínimo e rotação (SEC-AUT-9, SEC-CRI-3).
- Nos testes, **mock só do serviço externo** (servidor fake, por exemplo `wiremock`, com eventos assinados). Orthanc e NATS reais.

## 2. Webhooks recebidos (PIX, WhatsApp, NFS-e, assinatura)

Ordem obrigatória (G3):

1. Ler o **corpo bruto** e verificar a assinatura **antes** de qualquer parse ou processamento, em tempo constante (SEC-CRI-2).
2. Gravar o **ID do evento** em tabela com chave única **antes** de agir. Se já existir, responder 200 e parar.
3. Responder **2xx rápido**; o processamento segue pelo outbox.
4. **Não depender da ordem**: o worker busca o estado atual no provedor antes de decidir.
5. Produtor externo nunca vira comando direto: validação, assinatura e regra de negócio (SEC-MSG-6).

```sql
CREATE TABLE integracoes.webhook_recebido (
  provedor     text NOT NULL,
  evento_id    text NOT NULL,          -- ID do provedor, como recebido
  recebido_em  timestamptz NOT NULL DEFAULT now(),
  corpo_hash   bytea NOT NULL,
  processado   boolean NOT NULL DEFAULT false,
  PRIMARY KEY (provedor, evento_id)
);
```

Teste obrigatório: webhook PIX duplicado não duplica recebimento (`docs/27` §7); webhook com assinatura forjada é recusado.

## 3. DICOM e aparelhos

- **PACS no Orthanc** (C++), com plugin PostgreSQL e DICOMweb; visualizador OHIF (uso diagnóstico depende da avaliação
  regulatória, PREM-18).
- Perfis **IHE SWF** (pedido, worklist, MPPS, imagem) e **PIR** (correção de identificação). Integração fora do perfil exige ADR.
- Nenhum aparelho é integrado sem o **DICOM Conformance Statement** do fabricante; o VidaCor publica o seu.
- Só **AE Title e IP cadastrados**, em VLAN própria; limite de tamanho por associação; validação dos dados do estudo (SEC-MSG-7).
- Fluxo (docs/11 §6): check-in cria pedido com accession; item vai à worklist (MWL); aparelho envia MPPS em andamento,
  imagens (C-STORE) e SR; gateway publica `EstudoIniciado`, `ImagemRecebida`, `MedidasRecebidas`; conciliação confere
  paciente e estudo; MPPS concluído coloca o exame na fila de laudo com as medidas do SR no rascunho.
- **Estudo sem paciente conciliado nunca vai para laudo.** Divergência vai para a fila de revisão; vínculo manual só
  por médico, com dois identificadores e auditado.
- Lastro do estudo: StudyUID, séries, SOPs e **hash**, dose (RDSR), lote de contraste, desinfecção de transdutor (docs/21 §3.5).
- Contingência: Patient ID e Accession no aparelho recebem o código da etiqueta (`C-NNNN`) e os estudos vão para a fila
  própria de conciliação (`docs/23` §7.3 e §8.3).
- `dicom-rs` serve para utilitários e gateway; parsers cobertos por fuzzing e sem pânico (SEC-MEM-3); descompressão
  limitada (SEC-DIS-3).

## 4. Faturamento TISS e autorização

- XML com `quick-xml`/`serde`, **entidades externas e DTD desabilitadas**, limites de tamanho e profundidade (SEC-INJ-3).
- **Validação XSD** da ANS: binding `libxml2` em crate de FFI revisado (único lugar com `unsafe`, SEC-MEM-1) ou validação no
  serviço JVM. Escolha a confirmar por ADR no início da Fase 3; critério: validação conferida contra a massa de testes
  da suíte Clinica.
- Pedido de autorização (E4) usa os dados do agendamento, sem redigitação; resposta da operadora gravada como recebida;
  chave de idempotência por solicitação.
- Auditoria antes do faturamento (E5): regras por operadora **versionadas com vigência**; guia com erro bloqueante não
  entra em lote; a versão da regra aplicada é gravada.
- Reescrita em Rust no MVP depende da PREM-17 (a confirmar); a alternativa do ADR-0003 é rodar a partir da suíte Clinica
  atrás do mesmo contrato.

## 5. PIX, NFS-e e WhatsApp

- PIX por PSP a confirmar (PREM-10). Cobrança e estorno com chave de idempotência; estorno exige reautenticação e
  aprovação de outra pessoa (SEC-AUT-5, docs/18 §6).
- NFS-e pelo padrão nacional (PREM-12); solicitada uma vez por recebimento, pelo outbox.
- WhatsApp pela plataforma oficial (PREM-11); opt-out respeitado; mensagem com modelo versionado e status no lastro;
  nunca resultado de exame ou diagnóstico na mensagem.

## 6. Assinatura ICP-Brasil

- Serviço dedicado em JVM (`services/assinatura`, Kotlin, ADR-0008): PAdES para laudos e documentos, XAdES no TISS
  quando exigido; provedor de certificado a confirmar (`docs/11` §11 item 3, PREM-13).
- O núcleo envia o PDF e recebe o assinado; a versão assinada é gravada como imutável com certificado e carimbo do tempo.
- Assinar exige reautenticação (step-up). Só laudo assinado vai ao portal, e nunca o marcado "conversar antes".

## 7. Documentos (Typst)

- Laudos, atestados, recibos, etiquetas e preparo em `crates/documentos`, com modelos versionados no git.
- Dado do usuário entra **sempre escapado como texto**, nunca como código de modelo (SEC-INJ-4).
- Hash SHA-256 de todo documento gerado; versão do modelo gravada no lastro.
- Etiqueta só com nome, nascimento e códigos, nunca diagnóstico (`docs/20`). Reimpressão com motivo invalida a anterior.
- Arquivos temporários com permissão restrita e apagados (SEC-DSK-6); nomes gerados pelo servidor (SEC-INJ-5).

## 8. Workers de IA (Python)

- `services/ia`: transcrição de ditado, rascunho de laudo, leitura de placa (futuro). `ruff` e `mypy --strict`, sem estado.
- **IA sem autoridade de escrita**: não assina, não libera, não publica, não envia mensagem; devolve rascunho marcado que
  o médico revisa (SEC-IA-2, R11).
- Entrada tratada como não confiável (SEC-IA-1); contexto isolado por paciente e sessão (SEC-IA-3); saída renderizada
  como texto (SEC-IA-4); dado de saúde só vai a modelo externo com contrato e base legal, preferindo modelo local (SEC-IA-5).
- Uso diagnóstico de IA depende de avaliação regulatória (F2, Anvisa RDC 657/2022).
