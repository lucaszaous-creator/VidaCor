# CLAUDE.md — regras do projeto VidaCor

Este arquivo vale para **qualquer pessoa ou assistente de IA** que trabalhe neste
repositório. As regras abaixo são obrigatórias; o detalhamento, com exemplos e
testes exigidos, está em [docs/15-padrao-de-qualidade.md](docs/15-padrao-de-qualidade.md).
As regras gerais de arquitetura estão em [docs/14](docs/14-regras-de-arquitetura-e-engenharia.md)
e as decisões em [docs/adr/](docs/adr/).

## O projeto
Sistema da clínica de diagnóstico por imagem **VidaCor Imagem**: agenda, recepção,
caixa, atendimento médico, exames, aparelhos DICOM, laudos, imagens, faturamento
TISS, financeiro, estoque, WhatsApp, portal do paciente e estacionamento.
Servidor **local**. Núcleo em **Rust** (axum, sqlx), banco **PostgreSQL**, telas em
**TypeScript/React** (web e Tauri). Visão geral: [README](README.md) e [docs/11](docs/11-arquitetura-e-infraestrutura.md).

**Estado atual:** construção iniciada (Fase 0, esqueleto M0 do [docs/29](docs/29-esqueleto-m0.md)):
núcleo Rust com login OIDC, agenda do dia, check-in com dois identificadores, worklist no Orthanc
e conciliação de estudo, com testes de integração em PostgreSQL e Orthanc reais. O comando único de
verificação é `just test`; como rodar está no [README](README.md#como-rodar).

## Regra de decisão (obrigatória) — [ADR-0019](docs/adr/0019-decisoes-pelo-mais-profissional-com-lastro.md)
Na dúvida entre caminhos, escolha **o mais profissional e o mais rastreável** e registre em
[docs/decisoes.md](docs/decisoes.md): data, quem decidiu, alternativas, motivo, onde foi aplicado e como desfazer.
Nada é apagado para resolver. Só pergunte ao dono do produto o que depende de informação que só ele tem ou de
aprovação formal; mesmo assim, leve uma proposta pronta com premissas marcadas "a confirmar".

## As seis garantias (inegociáveis)

### G1 · O servidor decide; o cliente só pede
- Tudo que roda no navegador ou no Desktop pode ser lido e alterado pelo usuário.
- **Preço, desconto, cortesia, valor de guia, permissão, perfil e limite** são calculados e validados **no núcleo Rust**. O servidor recalcula tudo e ignora valores vindos do cliente.
- Esconder botão é conforto de UX, não segurança: toda rota verifica a permissão do ato.
- **Nenhum segredo em código de tela** nem em variável que vai para o navegador (`VITE_*`, configuração do Tauri). Só valores públicos por design (por exemplo, URL do servidor).
- O banco **nunca** é exposto diretamente (nada de PostgREST/API automática). No portal, que roda na nuvem, RLS por paciente é obrigatório como segunda barreira.

### G2 · Concorrência se resolve no banco
- Todo trecho que **lê, decide e grava** sobre recurso limitado (horário de sala, senha da fila, caixa aberto, lote de estoque, número de guia/lote TISS, link de convite, cortesia) precisa de proteção **no PostgreSQL**, nesta ordem de preferência:
  1. **constraint única** (índice único);
  2. **operação atômica** (`UPDATE … WHERE … AND condição` conferindo as linhas afetadas);
  3. **transação com `SELECT … FOR UPDATE`** quando a regra for mais complexa.
- **Proibido** lock em memória, lock em Redis com expiração ou decisão baseada em cache para garantir exclusividade. Infraestrutura nova para isso só com ADR explicando por que o banco não basta.
- Cada proteção tem teste com **2 requisições simultâneas** provando que só uma tem efeito.

### G3 · Repetir não duplica (idempotência)
- **Webhooks recebidos** (PIX, WhatsApp, NFS-e, provedor de assinatura): verificar a assinatura com o **corpo bruto**, antes de qualquer processamento; gravar o **ID do evento em tabela com chave única antes** de agir; se já existir, responder 200 e parar; **não depender da ordem** de chegada (buscar o estado atual no provedor); responder 2xx rápido e processar pelo outbox.
- **Chamadas que fazemos** a provedores (cobrança PIX, estorno, emissão de NFS-e, envio de lote TISS, assinatura, envio de WhatsApp): **chave de idempotência UUID** gerada uma vez por operação de negócio e reaproveitada nas retentativas.
- **Rotas de escrita do núcleo** que criam cobrança, recebimento, guia, agendamento ou laudo aceitam o header **`Idempotency-Key`** e devolvem a mesma resposta se a chave se repetir.
- **Nunca** usar dado pessoal (CPF, e-mail, telefone, nome) como chave de idempotência.

### G4 · Teste de integração de verdade
- Fluxos críticos são testados **chamando a rota HTTP real**, com **PostgreSQL real de teste** (Testcontainers) e **Orthanc/NATS reais** quando o fluxo usar. Mock **só de serviço externo** (PSP do PIX, Meta, prefeitura, provedor de assinatura, Memed).
- Cada fluxo cobre: **caminho feliz**, **erros esperados**, **repetição** (clique duplo, webhook duplicado, 2 requisições simultâneas), **limites** (zero, negativo, esgotado, expirado) e **acesso indevido** (usuário ou paciente A tentando ler/alterar dado de B).
- As frases de comportamento ("quando…, então…") são escritas em português e **aprovadas pelo dono do produto antes** dos testes.
- **Proibido** apagar, pular (`#[ignore]`, `skip`) ou afrouxar teste para passar, e proibido mockar o banco ou a regra testada. Teste que falha por bug real **continua falhando** e o bug vai para [docs/bugs.md](docs/bugs.md).
- **Nunca** apontar teste para banco de produção nem usar dado real de paciente fora de produção.

### G5 · Camadas separadas
- Tela (React/Tauri) **não** contém regra de negócio nem acessa banco ou provedor.
- Domínio (crates `dominio-*`) **não** conhece HTTP, SQL, Orthanc nem provedores: fala por *traits*.
- Adaptadores (banco, Orthanc, PIX, WhatsApp, NFS-e, assinatura) ficam na borda.
- Arquivo acima de **400 linhas** pede revisão de responsabilidade; acima de **600**, o PR precisa justificar ou dividir.
- Refatoração **não muda comportamento**: testes de caracterização antes, um passo por vez, mesmos nomes públicos, rotas e colunas; bug encontrado vai para a lista, não é corrigido junto.

### G6 · Plano antes, alteração depois
- Auditorias são **somente leitura**: não alteram arquivos, não rodam migration, não instalam nada.
- Correções começam por **plano escrito** e esperam o **OK** do dono antes de mexer em código.
- **Toda migration nova** é revisada por uma pessoa antes de rodar. Nunca apagar dados para "fazer funcionar"; dados duplicados que impeçam uma constraint são reportados com proposta de limpeza.
- Nunca desligar uma proteção existente para "fazer funcionar".
- Se não for possível verificar algo, escrever **"não verificado"** e o motivo, sem chutar.

## Proteção do usuário (obrigatória) — [docs/16](docs/16-protecao-do-usuario.md)
Nenhum código, tela ou lançamento ultrapassa estes caminhos:
- **Antes de construir:** documento de design, risco clínico no registro (risco alto sem controle não é construído), requisito com ID e teste.
- **Ao construir:** G1–G6; perfis IHE SWF/PIR com os aparelhos; **dois identificadores** (nome + nascimento) em todo ato clínico.
- **Ao testar:** usabilidade com a equipe nas tarefas críticas; caos em homologação.
- **Ao lançar:** revisão de prontidão completa, piloto com comparação automática, liberação gradual por chave, sem lançamento em véspera de feriado ou fechamento.
- **Na tela (R1–R14):** prevenir em vez de avisar; confirmação só em ato irreversível e com o nome do paciente; alertas em três níveis; corrigir com versão, nunca apagar; nada se perde; o atendimento nunca para; mensagem diz o que aconteceu, o que foi feito e o que fazer; a tela nunca mente; ética com o paciente (resultado delicado só após conversa, IA sempre revisada).
- **A tela cabe na janela:** telas de sistema em 1440×900 e 1280×800 sem rolar a página; o que é longo rola dentro do painel; o secundário vai para abas, detalhe ou gaveta ([docs/05](docs/05-design-system.md)).
- **Proibido "cara de IA":** emoji, caracteres como ícone (✓ ✕ ▶ ★ ⚡ ✨ →), raio/brilho/varinha, gradiente decorativo, pílula para tudo, correntes de "·", cards de "dicas/regras", tom de marketing ([docs/05](docs/05-design-system.md)).
- **Sem rolagem lateral:** em telas de 1280 px ou mais nenhuma tela obriga a arrastar para o lado; tabelas cabem na largura (menos colunas, texto quebrando, detalhe abaixo), sem `min-width` fixo ([docs/05](docs/05-design-system.md)). Exceções: telão e visualizador de imagens.
- **Menu de contexto (clique direito)** em todo objeto, seguindo o [docs/19](docs/19-menu-de-contexto.md): também por teclado (Menu/Shift+F10) e botão ⋯; ação que o perfil nunca terá não aparece, indisponível aparece apagada com motivo; destrutivas no fim com motivo; nada clínico em lote; ações vêm do servidor.
- Mudança em fluxo clínico precisa da aprovação do **responsável clínico de segurança**.

## Segurança por classe de ataque (obrigatória) — [docs/17](docs/17-seguranca-por-classe-de-ataque.md)
Baseado na skill **Cloudflare security-audit** (instalada em `.claude/skills/security-audit/`, MIT). Regras de construção `SEC-*` por domínio, entre elas:
- **Injeção:** SQL só parametrizado; XML sem entidades externas; Typst com dado escapado; caminhos de arquivo gerados pelo servidor.
- **Acesso:** negar por padrão; autorização **por objeto** (ter o ID não dá direito); teste de acesso indevido em toda rota com ID.
- **Autenticação:** OIDC com PKCE/state/nonce; JWT validado; sessão renovada e invalidada; cookies HttpOnly/Secure/SameSite + CSRF; 2FA sem atalho; **reautenticação** para assinar, estornar, autorizar desconto, exportar e mudar permissão; `no-store` em resposta com dado de saúde.
- **Telas:** sem `dangerouslySetInnerHTML`; CSP estrita e `frame-ancestors 'none'`; token e dado de saúde fora do armazenamento do navegador; WebSocket autorizado por evento.
- **Tauri:** capacidades mínimas; conteúdo remoto sem acesso à ponte; servidor decide permissão; atualizador assinado.
- **Mensageria/DICOM:** permissão por assunto no NATS; enum desconhecido rejeitado; ack depois de gravar; só aparelhos cadastrados.
- **Disponibilidade:** limites de tamanho, página, profundidade e tempo; regex linear; descompressão limitada; limite de tentativas antes do login.
- **Dados:** filtro de dono em cada consulta; cópias derivadas, logs e backups com as mesmas permissões; sem oráculo de enumeração; links assinados curtos; revogação vale para cache e fila.
- **Cripto, cadeia de suprimentos, infra, IA e memória:** sem cripto caseira; Actions por SHA e sem `pull_request_target`; contêiner sem root; consoles só na VLAN de administração; IA sem autoridade de escrita e com entrada tratada como não confiável; `unsafe` só em crate de FFI revisado.
- **Auditoria:** `/security-audit` antes da produção, de versões maiores e após incidentes; achado só com fronteira cruzada e resultado concreto; nunca testar contra produção ou dados reais.

## Perfis, permissões e RLS (obrigatório) — [docs/18](docs/18-perfis-permissoes-e-rls.md)
- Três camadas: **perfil** (atos) → **contexto** (unidade, sala, vínculo de cuidado, laudo atribuído, caixa próprio) no núcleo → **RLS forçado** no PostgreSQL com `SET LOCAL app.*` por transação.
- Médico vê dado clínico completo só com **vínculo de cuidado**; fora disso, **acesso de emergência** com justificativa, auditado e revisado.
- **TI sem dado clínico**; separação de funções (quem pede estorno não aprova; perfil clínico novo exige segunda pessoa).
- Toda política RLS tem teste de autorizado e não autorizado; proibido `USING (true)`, `BYPASSRLS` nos papéis da aplicação e `SET` de sessão para contexto.

## Lastro e rastreabilidade (obrigatório) — [docs/21](docs/21-lastro-e-rastreabilidade.md)
- **Nada existe solto:** todo registro tem `id` (UUIDv7 + código legível), criador, origem/canal, estação, unidade, versão, correlação, ligações tipadas, hash de arquivos, versão da regra aplicada e motivo quando há correção.
- **Eventos imutáveis** (append-only, hash encadeado) por domínio; nada é apagado.
- **Convênio só com guia autorizada:** o agendamento de convênio exige a seção da guia (carteirinha, nº da guia, senha, validade, TUSS autorizados e saldo, solicitante, anexos) com conferências automáticas; a guia é consumida de forma atômica e não pode ser usada por dois atendimentos; sem guia, só reserva provisória com prazo.
- Rastrear também: worklist/MPPS/estudo com hash, dose de radiação (RDSR), lote de contraste por paciente, desinfecção de transdutores, manutenção e controle de qualidade dos aparelhos, mensagens enviadas, leituras de dado clínico, configurações versionadas.
- Qualquer código leva à cadeia completa (explorador de lastro e dossiê assinado).
- Roteiro aprovado em [docs/22](docs/22-lacunas-e-roadmap.md): triagem de segurança bloqueante, laudo estruturado com versão da classificação, exame externo nunca vira laudo da VidaCor, autorização pedida pelo sistema e auditoria antes do faturamento com regras versionadas por operadora.

## Testes ponta a ponta das telas (obrigatório) — [docs/28](docs/28-testes-ponta-a-ponta-e2e.md)
Skill **e2e** (TesterArmy, Apache-2.0) instalada em `.claude/skills/e2e/`; regras `E2E-*`, entre elas:
- E2E é a camada de fumaça das **jornadas aprovadas** (frase `FC-*`); **não substitui** a integração da G4, e regra de negócio nunca é provada só pela tela.
- Resultado que importa com **`expect` exato**; `agent.assert` sozinho não prova regra clínica, financeira ou de permissão; dois identificadores conferidos em ato clínico.
- Acesso indevido pela tela em cada jornada; telas verificadas em 1440×900 e 1280×800 sem rolar a página.
- **Só dado fictício**, nunca produção; passo de agente envia a tela ao modelo (preferir modelo local, SEC-IA-5); segredos só em `credentials`/`secrets` vindos da CI; `.e2e/` fora do commit.
- Sem espera fixa (`sleep`); nada pulado ou afrouxado; cache de gravações em modo estrito na CI; `npx e2e feedback` só com aprovação do dono.
- Pacotes `e2e` e `@e2e-dev/*` com versão exata; skill atualizada só por commit de origem registrado e revisado.

## Time de desenvolvimento (skills por papel)
O trabalho segue um time com cinco papéis, cada um com skill em `.claude/skills/`:
**produto** (PO/PM), **ux-ui**, **desenvolvimento** (backend, frontend, desktop e mobile), **qa** e
**devops-seguranca**. A skill **time-vidacor** coordena: qual papel aciona cada trabalho, os portões P0 a P9 da ideia
à produção, quem assina cada um (RACI) e o modelo de passagem entre papéis.
- Comece pela `time-vidacor` quando não estiver claro qual papel usar; leia a skill do papel antes de agir nele.
- **Os papéis são aplicados pelo assistente na própria conversa, um de cada vez.** Proibido criar subagentes ou delegar a agentes por iniciativa própria: só quando o dono do produto pedir, no número que ele pedir.
- A IA prepara, a pessoa aprova: o assistente, em nenhum papel, aprova frase, risco clínico, ADR, migration, aceite, prontidão ou escopo.
- As skills não substituem os documentos: em conflito vale o documento, e a skill é corrigida no mesmo PR.

## Playbooks
Os roteiros de auditoria e correção ficam em [docs/playbooks/](docs/playbooks/) e
também como comandos do Claude Code em `.claude/commands/`:
`/auditoria`, `/testes-integracao`, `/refatoracao-segura`, `/seguranca`,
`/idempotencia`, `/condicao-de-corrida`.
Auditoria completa de segurança: `/security-audit` (skill da Cloudflare, ver docs/17).

## Regras clínicas que não podem ser quebradas
- Registro clínico (laudo, evolução, documento assinado) **nunca** é apagado nem alterado: correção gera nova versão com motivo.
- Laudo só vai ao portal **depois de assinado**.
- Dado de saúde **nunca** em log, métrica, URL ou cache do navegador.
- Imagem DICOM sem paciente conciliado **nunca** vai para laudo.
- **Etiquetas** ([docs/20](docs/20-etiquetas-e-rastreio.md)): código único por etiqueta, eventos imutáveis, reimpressão com motivo invalida a anterior, pulseira bipada na sala errada **bloqueia** o exame; etiqueta só com nome, nascimento e códigos (nunca diagnóstico).
