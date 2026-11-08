# @vidacor/api-client

Cliente HTTP fino do núcleo VidaCor, usado pelas telas (`apps/web`).

- Sessão em cookie HttpOnly (`credentials: 'include'`); nenhum token no navegador (SEC-CLI-3).
- `X-CSRF-Token` em toda escrita, com o valor de `GET /v1/sessao` guardado só em memória.
- `Idempotency-Key` UUID gerado uma vez por operação de negócio (`novaChaveDeIdempotencia`) e
  reaproveitado nas retentativas; escrita com chave é reenviada sozinha quando a rede cai (G3).
- Erros `application/problem+json` viram `ErroDaApi` com `codigo`, `detail` e `trace_id`.
- `abrirEventos` abre o SSE `GET /v1/eventos` e reabre com espera crescente.

## Fase 1

`src/contrato-fase1.ts` tem os tipos das rotas do docs/30, escritos à mão e marcados "A CONFIRMAR"
onde o docs/30 não descreve a forma. Serão conferidos contra o `openapi.json` gerado quando o núcleo
publicar essas rotas (como `src/conformidade.ts` já faz com o M0). Atos sensíveis respondem
`401 reautenticacao-necessaria` (`ehReautenticacao`); `sessao.urlDeReautenticacao(voltar)` monta a
ida ao provedor só com caminho da própria aplicação. Campos extras do Problem Details
(`sugestoes`, `sala_correta`, `pendencias`...) ficam em `erro.campo(nome)`.

## Tipos do contrato

Enquanto o núcleo não publica o `openapi.json`, os tipos estão escritos à mão em
`src/contrato-m0.ts`, copiados do docs/29 § 6.2.

`pnpm --filter @vidacor/api-client gerar` roda `openapi-typescript` sobre `openapi.json` e grava
`src/gerado/openapi.ts`. **Hoje falha**, porque o `openapi.json` ainda não existe; ele é gerado pelo
núcleo com `just contrato`. Quando existir, `src/contrato-m0.ts` passa a reexportar os tipos gerados.
