# Avisos de terceiros

| Componente | Origem | Licença | Onde |
|---|---|---|---|
| security-audit skill | [cloudflare/security-audit-skill](https://github.com/cloudflare/security-audit-skill), commit em `.claude/skills/security-audit/UPSTREAM_COMMIT` | MIT, © 2025-2026 Cloudflare, Inc. (texto em `.claude/skills/security-audit/LICENSE`) | `.claude/skills/security-audit/` |
| e2e skill | [tester-army/e2e](https://github.com/tester-army/e2e), commit em `.claude/skills/e2e/UPSTREAM_COMMIT` | Apache-2.0, © 2026 TesterArmy, Inc. (texto em `.claude/skills/e2e/LICENSE`, aviso em `.claude/skills/e2e/NOTICE`) | `.claude/skills/e2e/` |
| Modelo anatômico | BodyParts3D, © The Database Center for Life Science | CC BY-SA 2.1 Japan | `design/assets/anatomia/` (ver `docs/10-assets-e-licencas.md`) |
| Imagens DICOM de exemplo | Arquivos de teste da biblioteca pydicom | Ver `docs/10-assets-e-licencas.md` | `design/assets/imagens-exame/` |
| Motor de documentos Typst 0.15.1 (`typst`, `typst-layout`, `typst-pdf`) | [typst/typst](https://github.com/typst/typst), crates.io | Apache-2.0, © The Typst Project Developers | `crates/documentos/` (biblioteca no binário do núcleo) |
| Fontes embutidas nos PDFs (`typst-assets` 0.15.1): Libertinus Serif, New Computer Modern, DejaVu Sans Mono e as demais do pacote | [typst/typst-assets](https://github.com/typst/typst-assets), crates.io | SIL Open Font License 1.1 e as licenças listadas no arquivo `NOTICE` do pacote `typst-assets` (o crate em si é Apache-2.0) | Embutidas no binário do núcleo; o PDF gerado carrega só o subconjunto usado |
| Leitura DICOM `dicom-rs` 0.10.1 (`dicom-object`, `dicom-pixeldata` e dependências) | [Enet4/dicom-rs](https://github.com/Enet4/dicom-rs), crates.io | MIT ou Apache-2.0, © Eduardo Pinho e colaboradores | `crates/visualizador/` |
| OpenJPEG (JPEG 2000, pelo `openjpeg-sys`) | [uclouvain/openjpeg](https://github.com/uclouvain/openjpeg) | BSD-2-Clause, © Université catholique de Louvain e colaboradores | `crates/visualizador/` (compilado no binário) |
| CharLS (JPEG-LS, pelo crate `charls`) | [team-charls/charls](https://github.com/team-charls/charls) | BSD-3-Clause, © Team CharLS | `crates/visualizador/` (compilado no binário) |
| `jpeg-encoder` 0.7.1 | [vstroebel/jpeg-encoder](https://github.com/vstroebel/jpeg-encoder), crates.io | MIT ou Apache-2.0, e IJG para partes derivadas da libjpeg: "this software is based in part on the work of the Independent JPEG Group" | `crates/visualizador/` (entra pelo dicom-rs) |
