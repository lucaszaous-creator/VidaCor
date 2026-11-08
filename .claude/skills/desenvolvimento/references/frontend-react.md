# Frontend React e TypeScript

Fontes: ADR-0005; `docs/05-design-system.md`; `docs/11-arquitetura-e-infraestrutura.md` §4 e §9; `docs/14-regras-de-arquitetura-e-engenharia.md`
B3 e B9; `docs/16-protecao-do-usuario.md` Parte 2; `docs/17-seguranca-por-classe-de-ataque.md` B4; `docs/19-menu-de-contexto.md` §8.

## 1. Pilha

- TypeScript `strict: true`, **proibido `any`** (lint na CI), ESLint, Prettier, Vitest, Testing Library.
- React, Vite, TanStack Query (estado do servidor) e TanStack Router.
- Dados do servidor **só pelo cliente gerado** (`packages/api-client`); nada de `fetch` improvisado.
- Componentes do **design system** (`packages/design-system`, a partir de `design/assets/vidacor.css` e `design/vc10.css`)
  em vez de HTML improvisado.
- Aplicações: `apps/web` (equipe), `apps/portal` (paciente, na nuvem), `apps/desktop` (casca Tauri sobre a mesma web).

## 2. A tela não decide (G1, G5)

- Sem regra de negócio na tela: preço, desconto, permissão, limite, categoria de classificação e saldo de guia vêm do servidor.
- Esconder botão é conforto. Itens de menu e ações disponíveis vêm do servidor para aquele objeto.
- Validação de formato no cliente é só ajuda ao usuário; o servidor valida de novo.
- Nenhum segredo em código de tela nem em `VITE_*`. Só valores públicos por design (URL do servidor).

```ts
// Certo: o total vem do servidor; a tela só mostra.
const { data } = useQuery({
  queryKey: ['orcamento', orcamentoId],
  queryFn: () => api.orcamentos.obter({ id: orcamentoId }), // cliente gerado
});
// Errado: somar itens e aplicar desconto no componente e enviar o total.
```

## 3. Escrita com `Idempotency-Key` e sem perda (G3, R6)

- A chave nasce **uma vez por operação de negócio** (ao abrir o formulário de recebimento), não a cada clique.
- Botão de envio fica ocupado durante o pedido; reenvio após queda de rede usa a mesma chave.
- Rascunho salvo automaticamente em laudo, evolução, cadastro e recebimento (no servidor; nada de dado de saúde no navegador).

```ts
// Ilustrativo
const [chave] = useState(() => crypto.randomUUID()); // uma por operação
const receber = useMutation({
  mutationFn: (dados: NovoRecebimento) =>
    api.recebimentos.criar({ body: dados, headers: { 'Idempotency-Key': chave } }),
});
```

## 4. Segurança no navegador (docs/17 B4)

- Proibido `dangerouslySetInnerHTML` e HTML montado com dado de usuário (SEC-CLI-1). Texto de laudo e de IA como texto.
- CSP estrita sem `unsafe-inline`/`unsafe-eval`, Trusted Types, `frame-ancestors 'none'` (SEC-CLI-2).
- **Token nunca em `localStorage` ou `sessionStorage`**; sessão em cookie `HttpOnly`, `Secure`, `SameSite` com CSRF
  (SEC-CLI-3, SEC-AUT-4). Nada de dado de saúde em armazenamento do navegador; limpar tudo no logout e na troca de usuário.
- CORS com origens explícitas (SEC-CLI-4); `postMessage` com origem verificada; links externos com `rel="noopener noreferrer"`.
- Service worker não guarda resposta autenticada (SEC-CLI-7).
- Dado de saúde nunca na URL, no título da página ou em métrica de RUM.

## 5. Estrutura de tela (docs/05)

- Toda tela de sistema cabe em **1440×900** e funciona em **1280×800** **sem rolar a página**; o que é longo rola dentro do
  painel, com cabeçalho fixo. Secundário em abas, painel de detalhe ou gaveta.
- Estrutura: cabeçalho de uma linha (título, busca, ação principal), abas se houver, área de trabalho com 1 a 3 painéis.
  Modelo: `design/mockups/Pacientes-Lista.dc.html`.
- **Sem rolagem lateral** em 1280 px ou mais; tabela cabe na largura, sem `min-width` fixo. Exceções: telão e visualizador.
- **Barra do paciente** fixa no topo de toda tela clínica com nome e nascimento em destaque (`design/mockups/Barra-Paciente.dc.html`).
- **Estados** desenhados para todo bloco de dados: vazio, carregando, sem conexão, erro, sem permissão, salvamento automático
  e conflito, feito com desfazer, ato irreversível, campo com erro, busca sem resultado (`design/mockups/Estados.dc.html`).
- **Sem "cara de IA"**: sem emoji, sem caracteres como ícone, ícone é SVG de traço do conjunto do sistema, sem gradiente
  decorativo, chip só para status.

## 6. Requisitos de interface (docs/16 Parte 2) no código

| Requisito | Como aparece no código |
|---|---|
| R1 Prevenir | Horário ocupado não é clicável; ação indisponível apagada com motivo vindo do servidor |
| R2 Dois identificadores | Componente de cabeçalho clínico obrigatório; alerta de homônimo vindo do servidor |
| R3 Confirmação | Só em ato irreversível; texto com o nome e o nascimento do paciente, nunca "Tem certeza?" |
| R4 Alertas | Três níveis (bloqueante, atenção, informativo) como componentes distintos |
| R5 Desfazer | Desfazer por alguns segundos em ação comum; registro clínico só por nova versão |
| R7 Não parar | Lista de pendências visível quando um provedor externo falha |
| R8 Erros | Mostrar `detail` do Problem Details e o `trace_id` como código de suporte; falha nunca aparece como sucesso, ausência de dado nunca aparece como zero |
| R10 Acessibilidade | WCAG 2.2 AA, teclado completo, foco visível, `aria-label` em botão só com ícone, alvo de 44 px |

## 7. Menu de contexto (docs/19 §8)

- Um único componente `MenuDeContexto` no design system, alimentado por uma lista de ações vinda do servidor
  (id, rótulo, atalho, estado, motivo, perigo, submenu). **A tela não decide quais ações existem.**
- Abre por clique direito, tecla Menu, Shift+F10 e botão de mais opções; cabeçalho com nome e nascimento do paciente.
- Ação que o perfil nunca terá não aparece; indisponível aparece apagada com motivo; destrutivas no fim, com motivo.
- Cada ação chama a mesma rota da ação equivalente fora do menu. **Nada clínico em lote.**

## 8. Tempo real e desempenho

- Eventos do servidor por WebSocket/SSE invalidam ou atualizam o cache do TanStack Query; reconexão com retomada.
- Web Vitals medidos por tela e versão (`docs/12` §3.6), sem dado de saúde.
- Atalhos documentados (Ctrl+K busca, F2 check-in, F9 ditado).

## 9. Testes

- Unidade de componente com Testing Library (acessibilidade por papel e nome, não por classe CSS).
- `axe` na CI; teclado e leitor de tela nas telas novas.
- Ponta a ponta com a skill `e2e` e as regras `E2E-*` do `docs/28` (só jornadas aprovadas, `expect` exato, só dado fictício,
  verificação em 1440×900 e 1280×800).
- Regressão visual das telas e dos PDFs a cada versão.
