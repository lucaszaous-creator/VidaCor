# ADR-0006 · PACS com Orthanc e visualização com OHIF

- **Status:** proposto
- **Data:** 2026-11-06

## Contexto
É preciso receber imagens dos aparelhos (C-STORE), fornecer worklist (MWL), receber MPPS e SR, e visualizar estudos.

## Decisão
Orthanc com plugin PostgreSQL e DICOMweb como PACS; scripts Lua do Orthanc e gateway em Rust (dicom-rs) para transformar eventos DICOM em eventos de domínio e conciliar paciente × estudo. OHIF embutido como visualizador.

## Consequências
Padrão de mercado, open source e testado com muitos fabricantes. O uso do visualizador para diagnóstico depende de avaliação regulatória (Anvisa, RDC 657/2022).
