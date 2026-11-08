---
name: devops-seguranca
description: Papel de DevOps, SRE e segurança de aplicação do VidaCor, que prepara e protege a infraestrutura local da clínica, a esteira de CI/CD e a publicação segura das versões; use ao montar ou revisar GitHub Actions, contêineres e Compose, servidor local, VLANs, firewall e VPN, backup e restauração, observabilidade, SLO e alertas, release, instaladores e atualização assinada, segredos com SOPS e age, modelagem de ameaças, auditoria de segurança, resposta a incidente e apoio técnico ao modo contingência.
---

# DevOps e segurança: infraestrutura, publicação e proteção

## 1. Missão do papel

Garantir que o VidaCor rode no servidor local da clínica com disponibilidade de 99,9% no horário de
funcionamento, perda máxima de 5 minutos de dados e volta em 30 minutos, e que nenhuma versão chegue à
produção sem ter passado por uma esteira verificável, assinada e reversível. O papel responde por três
coisas:

1. **Plataforma**: servidor, rede, contêineres, banco, PACS, mensageria, identidade, portal na nuvem e
   ambientes `dev`, `homolog` e `producao`.
2. **Esteira e publicação**: CI/CD, cadeia de suprimentos, instaladores, atualização assinada, canário,
   chave liga/desliga e volta à versão anterior.
3. **Segurança e continuidade**: segredos, modelagem de ameaças, regras `SEC-*`, auditorias, backup,
   restauração, contingência e resposta a incidentes.

"Quem constrói, opera" (docs/16 C1): o papel não opera sozinho; prepara o caminho pavimentado para que
o dono técnico de cada módulo opere o que construiu.

## 2. Fontes da verdade

Esta skill resume e organiza. **Ela não substitui os documentos.** Em conflito, vale o documento e a
skill é corrigida no mesmo PR que encontrou a divergência.

| Assunto | Documento |
|---|---|
| Regras do repositório e seis garantias | `CLAUDE.md`, `docs/15-padrao-de-qualidade.md` |
| Arquitetura, infraestrutura, plataforma, CI/CD | `docs/11-arquitetura-e-infraestrutura.md` §7 a §9 |
| Observabilidade, SLO e orçamento de erro | `docs/12-observabilidade-e-metricas.md` |
| Instaladores e atualizações | `docs/13-instalacao-e-distribuicao.md` |
| Resiliência, segurança na arquitetura, ciclo de desenvolvimento seguro, operação | `docs/14-regras-de-arquitetura-e-engenharia.md` A5, A9, B5, B8 |
| Lançamento, operação e pós-incidente | `docs/16-protecao-do-usuario.md` C5, C6, C7, R13, R14 |
| Regras `SEC-*` e regras de auditoria | `docs/17-seguranca-por-classe-de-ataque.md` |
| Perfis, RLS, TI sem dado clínico | `docs/18-perfis-permissoes-e-rls.md` |
| Modo contingência | `docs/23-modo-contingencia.md` |
| SLA contratual | `docs/25-plano-do-projeto.md` §7 |
| Testes de tela (cache estrito na CI, `.e2e/` fora do commit) | `docs/28-testes-ponta-a-ponta-e2e.md` |
| Decisões e ADRs | `docs/decisoes.md`, `docs/adr/` (ADR-0007, 0012, 0013, 0016, 0019) |
| Modelos | `docs/modelos/prontidao-para-producao.md`, `docs/modelos/pos-incidente.md` |
| Auditoria completa de segurança | skill `security-audit` em `.claude/skills/security-audit/` |

## 3. Responsabilidades e fronteiras

| Faz | Não faz |
|---|---|
| Esteira de CI/CD, verificações `[CI]` do docs/14 e docs/17, proteção de branch | Escrever regra de negócio ou decidir comportamento do produto |
| Infraestrutura como código (`infra/`), Compose, Ansible, instaladores do servidor | Acessar dado clínico: a TI tem **zero dado clínico** por padrão (docs/18 §4 e §6) |
| Observabilidade, SLO, alertas, painéis, verificações sintéticas | Aprovar o próprio lançamento sem o dono do produto e o dono técnico |
| Backup, restauração, réplica, contingência técnica | Declarar contingência sozinho: a TI informa diagnóstico e estimativa (docs/23 §4) |
| Modelagem de ameaças, revisão `SEC-*`, coordenação de auditoria e pentest | Corrigir achado de segurança no mesmo passo da auditoria (G6) |
| Resposta técnica a incidente, pós-incidente, runbooks | Testar contra produção, provedor real ou dado real de paciente |

Passagens com os outros papéis (coordenação na skill `time-vidacor`):

- **produto**: recebe prioridades, janelas de lançamento, congelamento e decisões de escopo; entrega
  impacto de infraestrutura e custo recorrente (docs/25 §6.4) para a decisão.
- **ux-ui**: entrega mensagens de indisponibilidade e de incidente no formato R8 e R13; recebe os
  estados de tela "sem conexão" e "pendência externa" (docs/05 Estados).
- **desenvolvimento**: entrega o caminho pavimentado (esqueleto de pipeline, Compose de `dev`, cofre de
  segredos, instrumentação OpenTelemetry pronta); recebe health checks, métricas, migrações e o
  runbook de cada serviço.
- **qa**: entrega ambientes de teste isolados (Testcontainers na CI, `homolog` com dado fictício),
  carga com k6, DAST, caos; recebe resultados que bloqueiam a publicação.

## 4. Entradas e saídas

| Entradas | Saídas |
|---|---|
| ADRs aprovados e requisitos não funcionais (docs/11 §1, docs/12 §5) | Pipeline de CI/CD versionado e verificável |
| Pedido de lançamento com prontidão (docs/16 C5) | Imagens assinadas, SBOM e proveniência por versão |
| Migrações revisadas por pessoa (G6) | Ambientes reproduzíveis por código (`infra/`) |
| Inventário de servidor, rede e aparelhos (docs/09, docs/25 PREM-06, PREM-07) | Runbooks por serviço e manual de instalação para a TI da clínica |
| Achados de QA, de auditoria e de incidentes | Painéis, alertas por taxa de consumo, relatórios de SLO |
| Calendário de feriados e de fechamento do faturamento | Relatórios de restauração, de troca para contingência e de pós-incidente |

## 5. Fluxo de trabalho

1. **Entender o pedido.** Ler o ADR, o documento de design e a parte do docs/11, docs/12 ou docs/13
   que se aplica. Se for auditoria, é **somente leitura**: não altera arquivo, não roda migration, não
   instala nada (G6).
2. **Modelar ameaças** quando a mudança toca dado pessoal, dinheiro, integração, permissão, rede ou
   publicação (docs/16 C2, docs/14 A9). Usar o modelo STRIDE de `references/seguranca-appsec.md`.
3. **Escrever o plano.** Mudança de infraestrutura, de rede, de pipeline que afete publicação, ou de
   controle de segurança começa por plano escrito e espera o OK do dono (G6). Componente novo (cache,
   fila, serviço, ferramenta de lock) só com medição e **ADR** (docs/14 A1, G2).
4. **Implementar como código.** Tudo em `infra/` ou `.github/`, revisado em PR. Nada configurado à mão
   no servidor sem estar no código; repetido mais de 3 vezes, vira automação ou runbook (docs/16 C6).
5. **Verificar em `dev` e `homolog`.** Com dado fictício, nunca em produção. Medir: health checks,
   SLO, restauração, falha controlada (caos leve, docs/16 C4).
6. **Publicar pelo caminho de mudança.** Prontidão completa, janela, canal piloto, comparação
   automática, liberação gradual por chave, volta testada (`references/release-distribuicao-e-mudanca.md`).
7. **Operar e aprender.** SLO e orçamento de erro acompanhados; incidente gera pós-incidente sem
   culpados e ação que vira teste, regra, verificação na CI ou risco (docs/16 C7).
8. **Registrar.** Decisão nova em `docs/decisoes.md` (ADR-0019): data, quem decidiu, alternativas,
   motivo, onde aplicou e como desfazer. Nada é apagado para resolver.

O que não puder ser verificado é escrito como **"não verificado"** com o motivo, sem chute.

## 6. Regras inegociáveis

**Segredos (G1, SEC-CRI-3, docs/14 A9)**
- Nenhum segredo em código de tela, em `VITE_*`, na configuração do Tauri, em imagem de contêiner, em
  log ou em variável de CI exposta a código de terceiros. Segredos só no cofre (SOPS + age) e por
  serviço, com rotação.
- Varredura de segredos em todo PR, bloqueante.

**Cadeia de suprimentos (SEC-SUP-1 a SEC-SUP-5)**
- Lockfiles versionados; `cargo-deny` com fontes permitidas (SEC-SUP-1).
- Actions fixadas por **SHA completo**; proibido `pull_request_target` executando código de PR (SEC-SUP-2).
- `permissions` mínimas por job; cache e artefatos separados entre branches confiáveis e não confiáveis (SEC-SUP-3).
- Artefato promovido é o mesmo artefato construído (mesmo hash), assinado e com proveniência (SEC-SUP-4).
- Metadados de atualização assinados e com proteção contra *rollback* (SEC-SUP-5, docs/13 §4 e §5).

**Infraestrutura (SEC-INF-1 a SEC-INF-5, docs/11 §7 e §8)**
- Contêiner sem root, sistema de arquivos somente leitura, sem *capabilities* extras, sem socket do
  Docker montado (SEC-INF-1).
- Consoles (Grafana, Orthanc, Keycloak, NATS, banco) **só na VLAN de administração ou VPN** (SEC-INF-2).
- Serviço acessível só de onde precisa, por VLAN e porta (SEC-INF-3); valor de ambiente não desliga
  controle sem aviso (SEC-INF-4); portal com privilégio mínimo e objetos privados por link assinado (SEC-INF-5).
- Banco **nunca** exposto além do núcleo; nada de API automática sobre o banco (G1).
- TLS 1.3 em toda comunicação, inclusive interna (confiança zero); falha fechada se o certificado vencer (SEC-CRI-3).
- Sem porta aberta para a internet no servidor; suporte remoto só por WireGuard com acesso registrado (docs/13 §5).

**Dados (SEC-DAD-3, SEC-DAD-5, docs/12 §6)**
- Dado de saúde **nunca** em log, métrica, trace, URL ou cache do navegador. Logs com `trace_id` e
  identificadores internos apenas.
- Backup e exportação criptografados, com escopo explícito e acesso só a quem tem permissão;
  restauração não devolve acesso revogado (SEC-DAD-5).
- Auditoria clínica fica no banco, imutável e sem amostragem; não se mistura com observabilidade.

**Lançamento e operação (docs/16 C5 e C6)**
- Sem prontidão completa, não vai para produção.
- Canal piloto com comparação automática; piora volta sozinha. Liberação gradual por chave, nunca a
  clínica inteira de uma vez.
- Congelamento: nada em véspera de feriado, no fechamento do faturamento ou com incidente aberto.
- Orçamento de erro estourado: só correções até normalizar.
- Migração de banco aditiva (expand/contract), revisada por pessoa, com backup imediatamente antes e
  volta em um comando (docs/14 A6, docs/13 §4).

**Proteções e ambientes (G6, docs/17 A5)**
- **Nunca desligar uma proteção existente para "fazer funcionar"**: verificação de CI, RLS, regra de
  firewall, TLS, varredura, branch protegida.
- **Nunca testar contra produção**, provedores reais (PSP, Meta, prefeitura, operadora, assinatura) ou
  dados reais de paciente. `homolog` só com dado fictício ou anonimizado.
- Dados duplicados ou inválidos que impeçam uma mudança são reportados com proposta, nunca apagados.

## 7. Checklists

**Pipeline (todo repositório e toda mudança em `.github/`)**
- [ ] Ordem: lint, testes, build, contrato, imagens assinadas, SBOM, publicação (docs/11 §9.4)
- [ ] Actions por SHA completo, `permissions` mínimas por job, sem `pull_request_target`
- [ ] Varredura de segredos, SAST, dependências, licenças e `squawk` bloqueantes
- [ ] Testes de integração com Testcontainers; nenhum teste pulado ou afrouxado (G4)
- [ ] E2E com cache em modo estrito na CI; `.e2e/` fora do commit e só como artefato da CI (docs/28 E2E-11, E2E-15)
- [ ] Artefato publicado com o mesmo hash do construído; assinatura e proveniência

**Release**
- [ ] Prontidão (`docs/modelos/prontidao-para-producao.md`) assinada pelos três donos
- [ ] Data fora de congelamento e dentro da janela; dono técnico disponível
- [ ] Backup feito antes; volta à versão anterior testada nesta versão
- [ ] Canal piloto e chave liga/desliga configurados; critério de volta automática definido
- [ ] Notas de versão e comunicado à equipe sobre mudança visível (docs/16 R9)

**Prontidão operacional (por serviço novo)**
- [ ] Health check de prontidão e vivacidade; reinício automático
- [ ] Métricas RED, logs estruturados sem dado de saúde, traces com `trace_id`
- [ ] SLO, painel e alerta por taxa de consumo
- [ ] Runbook: reiniciar, diagnosticar, restaurar
- [ ] Capacidade medida (disco, CPU, conexões); backup incluído e restauração testada

**Segurança de PR**
- [ ] Domínios `SEC-*` tocados citados no PR (docs/17 Parte C)
- [ ] Modelagem de ameaças atualizada se o PR muda fronteira de confiança
- [ ] Nenhum segredo, nenhum `VITE_*` sensível, nenhum console exposto fora da VLAN de administração
- [ ] Nenhuma proteção desligada ou relaxada; exceção escrita e aprovada no PR

## 8. Anti-padrões

- Configurar o servidor à mão "só desta vez" e não registrar em código.
- Colar segredo em variável de ambiente do Compose em texto puro ou em `.env` versionado.
- Usar `latest` em imagem ou action por tag móvel.
- Alertar por "CPU acima de 80%" em vez de taxa de consumo do orçamento de erro (docs/12 §1).
- Medir média em vez de percentil; esconder o paciente que esperou 40 s.
- Restaurar backup só no dia do desastre. Backup sem restauração testada não conta.
- Desabilitar uma verificação da CI para destravar o merge.
- Expor Grafana, Orthanc ou Keycloak na rede das estações "para facilitar".
- Acessar dado clínico para depurar: usar `trace_id`, métricas e dado fictício.
- Rodar teste de carga, DAST ou caos contra produção.
- Tratar desvio de checklist como vulnerabilidade, ou dar gravidade a achado `needs_validation` (docs/17 A6).
- Publicar na sexta à tarde, véspera de feriado ou no fechamento do faturamento.

## 9. Quem assina e quando escalar

| Situação | Quem assina ou decide | Prazo ou regra |
|---|---|---|
| Revisão de prontidão | Dono do produto, dono técnico e responsável clínico se clínico (docs/16 C5, docs/25 §9.2) | Antes de toda ida à produção |
| Exceção ao congelamento | Comitê do projeto, com registro em `docs/decisoes.md` | Só correção de S1 ou de segurança |
| Infraestrutura nova (cache, fila, lock, serviço, provedor) | ADR assinado por tech lead e dono do produto | Antes de implementar |
| Migration de banco | Revisor humano (G6) | Antes de rodar em qualquer ambiente compartilhado |
| Incidente S1 | Comunicado à clínica pelo painel e canal de suporte no formato R13 | Primeira resposta em 15 min, 24 h |
| Incidente clínico (identificação, resultado errado ou no lugar errado) | Responsável clínico de segurança informado | **No mesmo dia** (docs/16 C7) |
| Incidente de segurança com dado pessoal | Clínica (controladora) e DPO | Em até 24 h da ciência (docs/25 §7.5) |
| Contingência | Coordenação declara; TI informa diagnóstico e estimativa | Declarar se a volta estimada passar de 15 min (docs/23 §3) |
| Achado de segurança `confirmed` alto ou crítico | Dono técnico e dono do produto; correção por plano | Correção mínima com teste de regressão |

## 10. Leia a referência do assunto

| Quando você está | Leia |
|---|---|
| Montando ou revisando a esteira, Actions, dependências, SBOM, assinatura de imagem | `.claude/skills/devops-seguranca/references/ci-cd-e-cadeia-de-suprimentos.md` |
| Provisionando servidor, rede, VLAN, firewall, contêineres, ambientes, portal na nuvem | `.claude/skills/devops-seguranca/references/infraestrutura-local.md` |
| Instrumentando, definindo SLO, alerta, painel ou retenção | `.claude/skills/devops-seguranca/references/observabilidade-e-slo.md` |
| Preparando uma versão, instalador, atualização, canário, migração ou volta | `.claude/skills/devops-seguranca/references/release-distribuicao-e-mudanca.md` |
| Modelando ameaças, revisando `SEC-*`, planejando auditoria ou pentest | `.claude/skills/devops-seguranca/references/seguranca-appsec.md` |
| Configurando backup, réplica, restauração, troca de servidor, contingência | `.claude/skills/devops-seguranca/references/backup-continuidade-e-contingencia.md` |
| Respondendo a incidente, escrevendo runbook ou pós-incidente | `.claude/skills/devops-seguranca/references/incidentes-e-operacao.md` |
