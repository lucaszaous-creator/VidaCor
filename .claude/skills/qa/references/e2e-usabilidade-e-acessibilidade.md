# Ponta a ponta, usabilidade e acessibilidade

Fonte: `docs/28-testes-ponta-a-ponta-e2e.md` (E2E-1 a E2E-18), skill `.claude/skills/e2e/`,
`docs/26-teste-de-usabilidade.md`, `docs/16` C4 e R10, `docs/05-design-system.md` (acabamento e
acessibilidade), `docs/19-menu-de-contexto.md` § 8.

## 1. Lugar do e2e

O e2e é a camada de fumaça das **jornadas aprovadas**. Ele confere o que a tela mostra do resultado;
a regra (preço, permissão, guia, concorrência) é provada no G4 (E2E-2). Antes de escrever um e2e,
confirme que o teste G4 da mesma frase existe.

Jornada de referência para o primeiro e2e (frases FC-AGD, check-in, pulseira, laudo, portal),
seguindo o protótipo `design/mockups/Fluxo-Completo.dc.html`: agendar convênio com guia, check-in
com dois identificadores, bipe da pulseira na sala, laudo com assinatura, portal.

## 2. Regras que QA confere em todo e2e

| Regra | Conferência |
|---|---|
| E2E-1 | Nome ou `tag` do teste traz o ID `FC-*` |
| E2E-3 | Cada resultado que importa tem `expect` com localizador por papel e nome e valor exato |
| E2E-4 | Ato clínico confere nome e nascimento na tela |
| E2E-5 | Existe o caso de acesso indevido pela tela (ação ausente ou apagada com motivo; rota forçada mostra negado) |
| E2E-6 | Roda em 1440×900 e 1280×800 e falha se a página rolar na vertical ou na horizontal |
| E2E-7 a E2E-11 | Só dado fictício; agente só em teste; segredos em `credentials`/`secrets`; `.e2e/` fora do commit |
| E2E-13 a E2E-16 | Sem espera fixa; nada pulado; cache estrito na CI; objetivo `agent.act` reescrito passa por revisão |

## 3. Exemplo de e2e com expect exato

Baseado na skill `e2e` (`references/writing-tests.md` da skill). Nomes de rota e textos são da
proposta de tela e serão confirmados com o papel `ux-ui`.

```ts
// tests/check-in.e2e.ts  (FC-CHK-01: check-in com nome e nascimento conferidos)
import { test, expect } from 'e2e';

test('check-in confere nome e nascimento [FC-CHK-01]', { tag: ['FC-CHK-01'] }, async ({ app, agent, screen }) => {
  await app.open('/recepcao');
  await agent.act('abrir o check-in de Antônio Carlos Ribeiro');

  const barra = screen.getByRole('region', { name: 'Paciente em foco' });
  await expect(barra).toContainText('Antônio Carlos Ribeiro');
  await expect(barra).toContainText('15/06/1954');

  await screen.getByRole('button', { name: 'Confirmar nome e nascimento' }).click();
  await screen.getByRole('button', { name: 'Concluir check-in' }).click();

  await expect(screen.getByRole('status')).toContainText('Check-in concluído');
});
```

Note: o `agent.act` só navega; toda conferência é `expect`. Nada de `agent.assert` como prova.

Verificação "a tela cabe na janela" (pseudocódigo, a confirmar com a API de leitura do e2e ou com
Playwright direto):

```ts
// pseudocódigo
for (const [w, h] of [[1440, 900], [1280, 800]]) {
  await browser.setViewport({ width: w, height: h });
  const { sw, cw, sh, ch } = await browser.evaluate(() => ({
    sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth,
    sh: document.documentElement.scrollHeight, ch: document.documentElement.clientHeight,
  }));
  expect(sw).toBeLessThanOrEqual(cw);   // sem rolagem lateral
  expect(sh).toBeLessThanOrEqual(ch);   // sem rolar a página
}
```

Enquanto não há telas em código, a mesma regra é verificada nos mockups por
`tools/verificacao/verifica.sh` e `tools/verificacao/sem-rolagem-lateral.js`.

## 4. Como rodar e corrigir

Siga a skill `e2e` (`SKILL.md`, seções "Workflow: fixing a failed test" e "Rerunning a test while you
fix it"): rodar por `arquivo:linha`, `--action-timeout 5000` durante a correção, `--retries 0`,
cache ligado; antes do merge, uma execução com `--no-cache` (E2E-15). Ler o `trace.md` da falha
antes de rodar de novo.

## 5. Acessibilidade (WCAG 2.2 AA)

Automático:
- `axe` na CI em toda tela nova e em cada estado desenhado (vazio, carregando, erro, sem permissão,
  conflito); zero violação de nível sério ou crítico.

Manual (por tela nova, registrado no PR):
- uso completo pelo teclado, ordem de foco lógica, foco visível, nenhuma armadilha de foco;
- atalhos do `docs/05` (Ctrl+K busca, F2 check-in, F9 ditado) funcionam e não conflitam;
- menu de contexto por Shift+F10 e tecla Menu, setas, Esc (`docs/19` § 8);
- leitor de tela anuncia botão só com ícone pelo `aria-label`, status por texto e não só por cor;
- zoom de 150% e 1366×768 sem perder função;
- alvo de toque mínimo de 44 px no tablet e no portal.

## 6. Menu de contexto (`docs/19` § 8)

Testes obrigatórios: teclado (Shift+F10, setas, Esc); leitor de tela; item indisponível aparece
apagado com motivo; item que o perfil nunca terá não aparece; destrutiva pede motivo e confirmação
com o nome do paciente; seleção múltipla sem ação clínica. As ações vêm do servidor: há também teste
G4 da lista de ações por perfil e estado.

## 7. Apoio ao teste de usabilidade (`docs/26`)

QA não modera nem anota a sessão; QA garante que o ambiente é confiável:
1. versão registrada (commit ou SHA-256 dos `.dc.html`, USA-03);
2. homologação com dados somente fictícios, conferida antes da rodada (USA-05);
3. estação em 1280×800 ou 1440×900, leitor de código de barras quando a tarefa pede bipe;
4. cenários do § 9 reproduzíveis (por exemplo, "pulseira errada" para T-TEC-02, homônimas para
   T-REC-04, guia vencida para T-REC-01);
5. todo achado `ACH-USA` que aponta comportamento errado do sistema vira caso de teste e, se for
   bug, linha em `docs/bugs.md`.

Critério que bloqueia: erro crítico em qualquer participante, ou menos de 4 de 5 sem erro e sem ajuda
(`docs/26` § 11, `docs/16` C4).
