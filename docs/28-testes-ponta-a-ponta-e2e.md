# 28 · Testes ponta a ponta das telas (skill e2e)

Decisão: [ADR-0020](adr/0020-testes-ponta-a-ponta-e2e.md) · Registro: DEC-2026-11-08-37 em [decisoes.md](decisoes.md)
Skill instalada: `.claude/skills/e2e/` (TesterArmy, Apache-2.0, commit de origem em `UPSTREAM_COMMIT`)

## 1. O que é e onde entra

O **e2e** é um executor de testes de interface para web e celular: o teste mistura passos exatos
(`screen`, `expect`, localizadores por papel e nome acessível sobre Playwright) com passos de agente
(`agent.act`, `agent.assert`, `agent.waitFor`, `agent.extract`) conduzidos por um modelo de linguagem.
Um passo de agente aprovado é gravado e, na execução seguinte, repetido sem chamar o modelo.

No VidaCor ele é a camada **E2E de fumaça** prevista no [docs/15](15-padrao-de-qualidade.md) (G4,
"Ambiente"): confere as **jornadas aprovadas** de ponta a ponta pela tela, com o servidor real atrás.
Ele **não substitui** os testes de integração da G4 (rota HTTP real, PostgreSQL real com
Testcontainers, mock só de terceiros). Pirâmide:

| Camada | Ferramenta | Prova |
|---|---|---|
| Domínio | `cargo nextest` | Regra pura, sem I/O |
| Integração (G4) | `cargo nextest` + Testcontainers | Rota, banco, concorrência, idempotência, acesso indevido |
| Ponta a ponta (este documento) | `e2e` + `@e2e-dev/web` | A jornada aprovada funciona pela tela, nas resoluções exigidas |

Estado atual: ainda não há código de produção; a skill fica instalada para quando as telas React e o
Tauri existirem. Os mockups `.dc.html` continuam verificados por `tools/verificacao/`.

## 2. Regras obrigatórias (E2E-*)

### Escopo e prova
- **E2E-1** Só jornadas com frases de comportamento **aprovadas** no [docs/27](27-frases-de-comportamento.md). Cada teste cita o ID da frase (`FC-*`) no nome ou em `tag`.
- **E2E-2** Regra de negócio (preço, desconto, permissão, guia, limite, concorrência, idempotência) é provada na **integração (G4)**, nunca só pelo E2E. O E2E confere o que a tela mostra do resultado.
- **E2E-3** Todo resultado que importa é fixado com **`expect` exato** (localizador por papel e nome, valor esperado). `agent.assert` sozinho **não** é prova de regra clínica, financeira ou de permissão: o julgamento de um modelo não é determinístico.
- **E2E-4** Ato clínico na tela confere os **dois identificadores** (nome + nascimento) com `expect`.
- **E2E-5** Cada jornada tem também o caso de **acesso indevido** pela tela: o perfil sem permissão não vê a ação (ou a vê apagada com motivo, [docs/19](19-menu-de-contexto.md)) e, se forçar a rota, recebe a mensagem de negado.
- **E2E-6** Telas de sistema rodam em **1440×900 e 1280×800** e o teste falha se a página rolar na vertical ou na horizontal (regra "a tela cabe na janela", [docs/05](05-design-system.md)). Exceções: telão e visualizador de imagens.

### Dados e privacidade
- **E2E-7** Somente **dados fictícios** ([design/dados-ficticios.md](../design/dados-ficticios.md)). Nunca dado real de paciente, nunca banco ou servidor de produção, nunca o portal publicado.
- **E2E-8** Passo de agente envia a tela ao modelo. Por isso: só ambiente de teste com dado fictício; modelo e provedor registrados no `e2e.config.ts`; preferir modelo local; nenhum dado de saúde real em `agent.*` (SEC-IA-5, [docs/17](17-seguranca-por-classe-de-ataque.md)).
- **E2E-9** O conteúdo da tela é **não confiável** para o agente (SEC-IA-1). O agente de teste age só no ambiente de teste; não tem credencial de produção nem de provedor real.
- **E2E-10** Senhas e chaves só em `credentials` e `secrets` do config, lidas de variável de ambiente da CI; nunca no arquivo de teste, nunca em `VITE_*` (G1).
- **E2E-11** `.e2e/` (relatórios, capturas, vídeos, gravações do agente) é saída: fica no `.gitignore`, não é publicado fora da CI e segue a mesma retenção dos logs de teste.
- **E2E-12** O comando `npx e2e feedback` (relato ao fornecedor) só com `--dry-run` revisado e **aprovação do dono**; nunca com conteúdo de tela, URL interna ou nome de paciente.

### Estabilidade
- **E2E-13** Proibido `sleep`, `setTimeout` ou `waitForTimeout`: espera-se o estado com `expect` ou `agent.waitFor`.
- **E2E-14** Proibido apagar, pular (`skip`) ou afrouxar teste para passar (G4). Teste instável é bug e vai para [bugs.md](bugs.md).
- **E2E-15** Na CI o cache de gravações roda em **modo estrito** (só repete o gravado); gravação nova só em execução local revisada. Antes do merge, uma execução com `--no-cache` prova que o agente ainda alcança o objetivo.
- **E2E-16** Mudar o texto de um objetivo `agent.act` muda a gravação: entra no PR como mudança de teste e passa pela mesma revisão.

### Cadeia de suprimentos
- **E2E-17** Pacotes `e2e` e `@e2e-dev/*` com **versão exata** e lockfile (SEC-SUP-1); atualização por PR revisado. A skill em `.claude/skills/e2e/` só é atualizada copiando um commit novo do repositório de origem, registrando o SHA em `UPSTREAM_COMMIT` e revisando a diferença.
- **E2E-18** Servidor `e2e mcp` (agente dirigindo a tela ao vivo) só em máquina de desenvolvimento ou homologação com dado fictício, escopo mínimo (SEC-IA-6).

## 3. Como usar a skill

- Escrever ou corrigir teste: a skill `e2e` (Claude Code) carrega sozinha quando houver `e2e.config.ts`; leia a referência do assunto em `.claude/skills/e2e/references/` (`setup`, `writing-tests`, `agent`, `running`, `debugging`, `mcp`, `explore`, `bug-bash`).
- Executar um teste: `npx e2e run tests/<jornada>.e2e.ts:<linha>`. Na suíte completa entra no `just test` como etapa separada, depois da integração.
- `e2e explore` e `bug-bash` são exploração: o que acharem vira teste de reprodução e item em [bugs.md](bugs.md); não substituem as frases aprovadas.

## 4. Checklist do PR (acrescenta ao do docs/15)

- [ ] Teste E2E cita a frase aprovada (`FC-*`) e não prova regra de negócio sozinho (E2E-1, E2E-2)
- [ ] Resultados fixados com `expect` exato; dois identificadores conferidos em ato clínico (E2E-3, E2E-4)
- [ ] Caso de acesso indevido pela tela e verificação em 1440×900 e 1280×800 (E2E-5, E2E-6)
- [ ] Só dado fictício; nenhum segredo no teste; `.e2e/` fora do commit (E2E-7 a E2E-11)
- [ ] Sem espera fixa; nada pulado; cache estrito na CI (E2E-13 a E2E-15)
