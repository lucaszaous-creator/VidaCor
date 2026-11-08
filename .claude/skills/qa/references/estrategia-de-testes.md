# Estratégia de testes do VidaCor

Fonte: `docs/14-regras-de-arquitetura-e-engenharia.md` § B4, `docs/15-padrao-de-qualidade.md` G4,
`docs/28-testes-ponta-a-ponta-e2e.md` § 1, `docs/11-arquitetura-e-infraestrutura.md` § 9.3.

## 1. Pirâmide e o que cada camada prova

| Camada | Ferramenta | Prova | Não prova |
|---|---|---|---|
| Domínio (unidade) | `cargo nextest` nos crates `dominio-*` | Regra pura: cálculo de total, categoria TI-RADS, transição de estado, validação de tipo (`Cpf`, `CodigoTuss`) | Banco, permissão na rota, concorrência |
| Propriedade | `proptest` | Regra vale para todas as combinações (convênio, cortesia, caixa, arredondamento em centavos) | Integração |
| Integração (G4) | `cargo nextest` + Testcontainers (PostgreSQL, Orthanc, NATS) + fake externo | Rota real, transação, constraint, RLS, concorrência, idempotência, acesso indevido, eventos de lastro | Experiência na tela |
| Contrato | Diff do OpenAPI (`utoipa`) e dos JSON Schema de eventos | Mudança incompatível não entra sem nova versão | Comportamento |
| Tela (componente) | Vitest + Testing Library | Componente mostra estado certo (vazio, carregando, erro, sem permissão), acessível por papel e nome | Regra de negócio (não existe na tela, G5) |
| Ponta a ponta | skill `e2e` (Playwright por baixo) | Jornada aprovada funciona pela tela, nas resoluções exigidas | Regra de negócio sozinha (E2E-2) |
| Visual | Comparação de prints de tela e PDF | Laudo, recibo, etiqueta e tela não mudaram sem querer | Conteúdo semântico |
| Desempenho | `k6` | Orçamento p95 e p99 por rota (`docs/12` § 5) | Correção |
| Acessibilidade | `axe` na CI e teclado manual | WCAG 2.2 AA nas telas novas | Usabilidade real |
| Mutação | `cargo-mutants` | Os testes pegam erro de verdade | Cobertura de cenário ausente |
| Fuzzing | `cargo-fuzz` | Parser não entra em pânico com entrada malformada (SEC-MEM-3) | Regra |

Regra de bolso: se o teste quer saber "a regra está certa", domínio ou propriedade; se quer saber
"o sistema protege de verdade", integração G4; se quer saber "a pessoa consegue", e2e e usabilidade.

## 2. Cobertura mínima (`docs/14` B4)

| Alvo | Mínimo | Verificação |
|---|---|---|
| Crates de domínio | 80% de linhas | CI |
| `dominio-laudos`, faturamento, caixa, assinatura | 90% | CI |
| Requisitos com status "implementado" | 100% com teste na matriz | CI |

Cobertura é piso, não meta. Linha coberta por teste sem asserção relevante não conta na revisão:
use mutação para conferir.

## 3. Mutação

- `cargo-mutants` roda nos módulos críticos (laudos, faturamento, caixa, assinatura, identidade,
  agenda com guia) em agenda noturna ou antes do aceite, não a cada PR (custo alto).
- Mutante sobrevivente em regra crítica vira caso de teste novo ou justificativa escrita (mutante
  equivalente). Nunca se ignora sem registro.
- Ferramenta para TypeScript: a confirmar (proposta: Stryker apenas no pacote de formatação e
  validação de entrada da tela, se houver lógica relevante ali; a regra está no núcleo).

## 4. Propriedade

Bons candidatos no VidaCor:
- total do caixa: soma dos itens menos desconto nunca negativa; desconto acima do limite sempre
  exige autorização; arredondamento em centavos é determinístico (`docs/14` A3);
- saldo de guia: nenhuma sequência de consumos e devoluções deixa saldo negativo;
- datas: gravar em UTC e exibir em `America/Sao_Paulo` ida e volta não muda o instante;
- categorias de laudo estruturado: mesma entrada e mesma versão da tabela dão sempre a mesma
  categoria.

Pseudocódigo (API do proptest simplificada):

```rust
// pseudocódigo
proptest! {
    #[test]
    fn total_nunca_negativo(itens in vec(valor_centavos(), 1..10), desconto in 0u8..=100) {
        let r = calcular_total(&itens, Percentual(desconto), &tabela_v12());
        prop_assert!(r.total_centavos >= 0);
    }
}
```

## 5. Fuzzing

Alvos obrigatórios (`docs/14` B4, SEC-MEM-3): parser DICOM e SR no gateway, XML TISS (geração e
leitura), retorno de convênio, XML de NFS-e, corpo de webhook. Critério: sem `panic`, sem consumo
superlinear, limite de tamanho e profundidade respeitado (SEC-DIS-1, SEC-INJ-3). Corpus inicial com
arquivos de exemplo sintéticos (`design/assets/imagens-exame/` traz DICOM de teste da pydicom).
Todo crash encontrado vira teste de regressão com o arquivo mínimo.

## 6. Contrato

- OpenAPI gerado do código é a fonte da verdade; cliente TypeScript gerado na CI. Mudança
  incompatível em `/v1` reprova o PR.
- Eventos com campo `versao`; consumidor tolera campo novo; enum desconhecido é rejeitado
  (SEC-MSG-2). Há teste para os dois comportamentos.
- Serviços em outra linguagem (assinatura JVM, workers Python) têm teste de contrato contra o
  mesmo schema.

## 7. Regressão visual

- Telas: print em 1440×900 e 1280×800 das telas aprovadas, comparado a cada versão.
- PDFs: laudo, laudo estruturado, recibo, etiqueta, ficha de contingência, renderizados com dados
  fictícios fixos e comparados por imagem. Etiqueta conferida contra `docs/20` (só nome,
  nascimento e códigos; nunca diagnóstico).
- Diferença aprovada exige atualização da referência no mesmo PR, com motivo.

## 8. Dados de teste

- Somente sintéticos ou os fictícios de `design/dados-ficticios.md` (Sônia Maria Castro, 03/02/1962;
  Antônio Carlos Ribeiro, 15/06/1954; Juliana Sampaio Rocha, 12/04/1993; e os demais).
- Homônimos e duplicados de propósito existem na lista (Maria Helena Duarte 12/03/1968 e
  21/03/1968): use-os nos testes de dois identificadores e de duplicidade.
- Fábricas de dados por domínio, com valores explícitos no teste quando importam para a asserção.
- Proibido dado real de paciente fora de produção (G4, `docs/14` B4), inclusive copiado "só para
  testar a migração": migração usa instância isolada do `docs/24` § 7.1.

## 9. Testes instáveis

- Teste intermitente é bug: aberto em `docs/bugs.md` ou corrigido no mesmo dia (`docs/14` B4).
- Causas comuns e correção: ordem entre testes (isolar banco por teste ou por transação), relógio
  (relógio injetado), espera fixa (esperar o estado), porta fixa (porta aleatória).
- Proibido re-executar até passar e chamar de verde. Medir com repetição (`--repeat-each 10` no e2e,
  laço de N execuções no nextest) antes e depois da correção.

## 10. `just test`

Comando único de verificação (`CLAUDE.md`). Ordem proposta, a confirmar quando o esqueleto existir
(Fase 0): lint e formatação, unidade e propriedade, integração com Testcontainers, contrato,
componentes de tela, e2e de fumaça. Mutação, fuzzing longo, k6 completo e regressão visual completa
podem ficar em alvos separados (`just test-mutacao`, `just test-carga`), sempre rodados antes do
aceite e da prontidão.
