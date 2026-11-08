# Fase 0: arranque do projeto (marco M0)

Base: `docs/25-plano-do-projeto.md` § 3.1 (Fase 0, meses 1 a 2), `docs/11-arquitetura-e-infraestrutura.md`
§ 11 e `docs/09-proximos-passos.md`. Datas no calendário só depois de confirmar as premissas
(`docs/25` § 5).

## Entregas do M0

| Entrega | Papel R | Quem aprova | Situação de partida |
|---|---|---|---|
| Premissas PREM-01 a PREM-26 revisadas | produto | Dono do produto | Todas "a confirmar" |
| ADRs 0002 a 0018 revisados e aceitos ou ajustados | desenvolvimento | Tech lead e dono do produto | Maioria "proposto" (0019 e 0020 aceitos) |
| Frases FC-* do `docs/27` aprovadas | produto | Dono do produto; responsável clínico nas clínicas | Todas "aguardando aprovação" |
| Teste de usabilidade R1 com os mockups (`docs/26`) | ux-ui | Dono do produto; responsável clínico nas tarefas clínicas | Roteiro pronto |
| Inventário de aparelhos, servidor e volume de exames | devops-seguranca com desenvolvimento (DICOM) | TI da clínica | Pendente (`docs/09`) |
| Pedido formal de exportação aos fornecedores atuais | produto | Dono do produto e jurídico | Pendente (`docs/24` § 3) |
| Esqueleto do monorepo com a rota de ponta a ponta | desenvolvimento | Tech lead | Não iniciado |
| Pipeline de CI com as verificações [CI] desde o primeiro commit | devops-seguranca | Tech lead | Não iniciado |
| Ambiente de homologação com dados fictícios | devops-seguranca | Tech lead | Não iniciado |
| Plano de testes do esqueleto e da Fase 1 | qa | Tech lead | Não iniciado |
| Contrato e contrato de operador | produto | Direção da clínica e fornecedor | Proposta no `docs/25` § 7 |

## Ordem recomendada

1. **Semana 1:** `produto` leva as premissas e a lista de aprovações pendentes ao dono do produto; `desenvolvimento` e `devops-seguranca` revisam os ADRs 0002 a 0018 e listam o que precisa mudar; `devops-seguranca` envia o roteiro de inventário à TI da clínica.
2. **Semanas 1 e 2:** `ux-ui` prepara a rodada R1 de usabilidade (versão do material registrada, cenários, consentimento); `qa` revisa as frases FC-* quanto aos cinco casos (feliz, erro, repetição, limite, acesso indevido) e devolve lacunas ao `produto`.
3. **Semanas 2 a 4:** aprovação das frases em blocos por fluxo; ADRs aceitos; `devops-seguranca` monta a CI vazia já com lint, varredura de segredos, `cargo-deny`, actions por SHA e branch protegida.
4. **Semanas 3 a 6:** `desenvolvimento` monta o esqueleto do `docs/11` § 9.1 e a rota de ponta a ponta: login, agenda do dia, check-in, worklist no Orthanc e imagem de exemplo chegando na tela por WebSocket. `qa` escreve os primeiros testes G4 dessa rota com Testcontainers.
5. **Semanas 5 a 8:** homologação no ar; usabilidade R1 feita e achados priorizados; plano da Fase 1 aprovado na revisão quinzenal; M0 apresentado ao comitê.

## Esqueleto: o mínimo que já nasce certo

- `just test`, `just lint`, `just dev` funcionando.
- Crates `api`, `comum` e um `dominio-*` de exemplo com `#![forbid(unsafe_code)]`, sem I/O no domínio.
- Migrations com metadados de lastro (`docs/21` § 1), RLS forçado na primeira tabela com dado de paciente e teste autorizado e não autorizado.
- Rota com permissão declarada (negar por padrão, SEC-ACS-1), Problem Details, `trace_id`, `Idempotency-Key` na primeira rota de escrita.
- OpenAPI gerado e cliente TypeScript gerado na CI.
- Tela React com o design system mínimo (tokens do `docs/05`), estados de tela e verificação nas duas resoluções.
- OpenTelemetry ligado desde o primeiro serviço; nenhum dado de saúde em log.
- Teste e2e de fumaça da rota de ponta a ponta seguindo o `docs/28`.

## Critério de saída do M0

- Premissas, ADRs e frases com situação registrada (aprovada, ajustada ou recusada com motivo).
- Esqueleto com `just test` verde na CI e a rota de ponta a ponta demonstrada em homologação.
- Relatório da usabilidade R1 assinado.
- Inventário recebido ou pendência registrada com dono e data.
- Linha em `docs/decisoes.md` para cada decisão tomada na fase.
