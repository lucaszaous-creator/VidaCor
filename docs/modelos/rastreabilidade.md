# Matriz de rastreabilidade (IEC 62304, versão enxuta)

Todo requisito aponta para o código e para o teste que o comprova. A CI falha se um requisito marcado como "implementado" não tiver teste.

| Requisito | Descrição | Risco relacionado | Módulo/crate | Teste(s) | Status |
|---|---|---|---|---|---|
| REQ-PAC-001 | Todo ato clínico exige confirmação por dois identificadores | RC-001 | dominio-pacientes | `pacientes::dois_identificadores_*` | Planejado |
| REQ-EXA-001 | Estudo DICOM sem conciliação não entra na fila de laudo | RC-002 | dominio-exames | `exames::conciliacao_*` | Planejado |
| REQ-LAU-001 | Laudo assinado é imutável; correção cria nova versão com motivo | RC-006 | dominio-laudos | `laudos::imutavel_*` | Planejado |
