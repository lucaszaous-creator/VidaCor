# 17 · Segurança por classe de ataque (baseado no Cloudflare security-audit)

> **Status:** regra obrigatória (v0.1, nov/2026). Decisão no
> [ADR-0016](adr/0016-seguranca-por-classe-de-ataque.md). Resumo no [CLAUDE.md](../CLAUDE.md).
>
> **Origem:** [cloudflare/security-audit-skill](https://github.com/cloudflare/security-audit-skill)
> (licença MIT, © Cloudflare, Inc.). A skill completa está instalada em
> [`.claude/skills/security-audit/`](../.claude/skills/security-audit/), com a
> licença e o commit de origem (`UPSTREAM_COMMIT`).
>
> A skill é uma **metodologia de auditoria**. Este documento a transforma em duas
> coisas: **(1) regras de construção**, que impedem cada classe de ataque desde o
> primeiro código (Parte B), e **(2) regras de auditoria**, para quando e como
> auditar o VidaCor (Parte A).

---

## Parte A — Regras de auditoria de segurança

### A1. Quando auditar
| Momento | Modo da skill | Escopo |
|---|---|---|
| Pergunta ou revisão pontual de segurança num PR | **Guidance mode** (só a parte relevante) | O trecho em revisão |
| Antes do **primeiro uso em produção** | **Full audit**, perfil `deep` | Repositório inteiro |
| Antes de cada **versão maior** | Full audit, perfil `standard` | Diferença desde a última auditoria + módulos críticos |
| Depois de **incidente de segurança** | Full audit `scoped` | Módulo e fronteiras envolvidos |
| Módulo novo que toca dinheiro, dado clínico, identidade ou integração | Full audit `scoped` | O módulo |

A auditoria completa roda com `/security-audit` (a skill instalada) e as saídas
ficam **fora do repositório** (padrão da skill: `~/security-audit-skill/vidacor/run-N`).
Achados confirmados viram itens rastreados, com teste de regressão.

### A2. Princípios do auditor (obrigatórios em toda revisão de segurança)
1. **Fronteira e resultado:** um achado só existe quando se aponta quem tem menos confiança, o que ele envia, qual controle deveria barrar, qual fronteira é cruzada, quem é afetado e o **resultado concreto**. Desvio de checklist sem fronteira cruzada **não** é vulnerabilidade.
2. **Evidência local e limitada:** primeiro o caminho no código; depois, se preciso, um teste local mínimo (com dados fictícios) que pare no menor efeito que prova o problema.
3. **Respeitar o que não está no código:** firewall, proxy, VLAN, política de identidade e configuração do provedor são controles reais. Se não estão no repositório, o achado é `needs_validation` com o fato exato que falta, nunca um chute.
4. **Prioridade separada de certeza:** só achado `confirmed` recebe gravidade; `needs_validation` não tem gravidade.
5. **Menor correção efetiva:** a invariante que o código deve garantir, no último ponto de decisão confiável, com teste de regressão. A auditoria descreve a correção; não altera o código.

### A3. Veredictos
| Veredicto | Significado |
|---|---|
| `confirmed` | Caminho completo no código e resultado observado de forma limitada |
| `needs_validation` | Hipótese concreta bloqueada por um fato externo identificado; sem gravidade |
| `rejected` | Candidato refutado (fica registrado para não ser reaberto sem fato novo) |

### A4. Gravidade, com exemplos do VidaCor
| Gravidade | Definição (Cloudflare) | Exemplo no VidaCor |
|---|---|---|
| **Crítica** | Atacante sem login executa código, acessa o banco inteiro ou toma qualquer conta | Rota do portal sem autenticação que lista laudos de todos os pacientes |
| **Alta** | Controle explícito totalmente derrotado com consequência real | Paciente A lê o laudo do paciente B; recepção assina laudo; XSS armazenado no nome do paciente executando na tela do médico; webhook PIX forjado marca cobrança como paga |
| **Média** | Fronteira violada com alcance limitado ou pré-condições incomuns | Funcionário de um perfil vê dado cadastral de outro setor sem dado clínico |
| **Baixa** | Exposição de detalhe interno não secreto | Versão de biblioteca visível num cabeçalho |
| **Informativa** | Observação confirmada de impacto mínimo | Pré-requisito de um ataque maior |

Pergunta que separa alta de média: **o resultado derrota totalmente um controle explícito para uma ação com consequência real, ou só o enfraquece?**

### A5. Segurança da execução durante a auditoria
- Leitura do código é somente leitura.
- Build, teste ou execução de código do projeto só em **sandbox** sem rede externa, com ambiente vazio, alvo somente leitura e limites de CPU, memória e tempo. Sem sandbox, não executa: vira `needs_validation`.
- **Proibido** testar contra produção, serviços externos, provedores reais (PIX, Meta, prefeitura), dados reais de pacientes ou contas reais. Só dados, contas e segredos fictícios.

### A6. Erros proibidos na auditoria (anti-padrões da skill)
1. Apresentar desvio de checklist como vulnerabilidade.
2. Recomendar "defesa em profundidade" sem violação alcançável.
3. Testar em ambiente real quando a evidência local bastaria.
4. Adivinhar comportamento de provedor, proxy, navegador ou implantação.
5. Tratar autoridade legítima do próprio usuário como fronteira cruzada.
6. Descrever efeito mais forte do que o observado.
7. Entregar resultado só em prosa, que não pode ser deduplicado nem verificado.
8. Relatar de novo achado antigo inalterado.
9. Dar gravidade a `needs_validation`.
10. Escrever o relatório antes da verificação independente.

---

## Parte B — Regras de construção por classe de ataque

Cada regra tem um ID (`SEC-<domínio>-<n>`) para ser citada em PR, teste e auditoria.
**[CI]** = verificada automaticamente.

### B1. Injeção (ATTACK-CLASSES)
- **SEC-INJ-1** SQL só com parâmetros (`sqlx::query!`/`query_as!`). Proibido montar SQL com `format!` ou concatenação **[CI: lint]**.
- **SEC-INJ-2** Nenhuma execução de comando do sistema com dado de usuário. Se for inevitável (ex.: ferramenta DICOM), argumentos em lista, nunca via shell.
- **SEC-INJ-3** XML (TISS, retorno de convênio, NFS-e) com **entidades externas e DTD desabilitadas** (sem XXE) e limite de tamanho e profundidade.
- **SEC-INJ-4** Modelos Typst recebem dado do usuário **sempre escapado** como texto, nunca como código de modelo.
- **SEC-INJ-5** Caminhos de arquivo nunca vêm do usuário: nomes gerados pelo servidor (UUID); proibido `..` e caminho absoluto.

### B2. Controle de acesso (ATTACK-CLASSES + DATA-ISOLATION)
- **SEC-ACS-1** **Negar por padrão:** toda rota declara a permissão do ato; rota sem declaração não compila ou falha no teste de arquitetura **[CI]**.
- **SEC-ACS-2** **Autorização por objeto:** todo acesso por ID (paciente, laudo, recebimento, estudo) confere se o usuário pode **aquele** objeto. Ter o ID não dá direito ao objeto (proteção contra IDOR).
- **SEC-ACS-3** A consulta ao banco carrega o filtro de dono/escopo; checar depois de buscar não basta.
- **SEC-ACS-4** Toda rota com ID tem teste de **acesso indevido** (usuário A → objeto de B) (G4).

### B3. Autenticação, sessão e identidade (WEB-PROTOCOL-AND-AUTH)
- **SEC-AUT-1** OIDC com **PKCE**, `state` e `nonce` verificados; redirecionamento só para URLs cadastradas.
- **SEC-AUT-2** JWT validado com algoritmo fixo, emissor, audiência, expiração e assinatura; nunca `alg: none`, nunca confiar em claim sem validar.
- **SEC-AUT-3** Sessão **renovada** no login e na troca de perfil; **invalidada** no logout, na troca de senha e na remoção de permissão.
- **SEC-AUT-4** Cookies de sessão `HttpOnly`, `Secure`, `SameSite=Lax/Strict`, escopo mínimo; proteção **CSRF** em toda escrita com cookie.
- **SEC-AUT-5** **2FA sem rebaixamento:** não há caminho de login que pule o 2FA; **reautenticação (step-up)** para assinar laudo, estornar, autorizar desconto, exportar dados e mudar permissões.
- **SEC-AUT-6** Recuperação de conta e convite do portal com token **de uso único, curto e vinculado** à conta; mensagens que não revelam se o CPF/e-mail existe.
- **SEC-AUT-7** Cabeçalhos `Host` e `X-Forwarded-*` só são confiados vindos do proxy conhecido; URLs absolutas geradas a partir de configuração, não do cabeçalho.
- **SEC-AUT-8** Resposta com dado pessoal ou de saúde: `Cache-Control: no-store`; nenhum cache de proxy guarda resposta privada.
- **SEC-AUT-9** Chaves de API de integração com escopo mínimo, vinculadas ao recurso e rotacionáveis.

### B4. Navegador e telas (CLIENT-SIDE)
- **SEC-CLI-1** Proibido `dangerouslySetInnerHTML` e HTML montado com dado de usuário **[CI: lint]**; texto de laudo e de IA é renderizado como texto.
- **SEC-CLI-2** **CSP estrita** (sem `unsafe-inline`/`unsafe-eval`), Trusted Types, `frame-ancestors 'none'` (contra clickjacking).
- **SEC-CLI-3** Token de acesso nunca em `localStorage`/`sessionStorage`; nada de dado de saúde em armazenamento do navegador; tudo limpo no logout e na troca de usuário.
- **SEC-CLI-4** CORS com lista explícita de origens; nunca `*` com credenciais.
- **SEC-CLI-5** WebSocket/SSE: autenticação no handshake, verificação de `Origin` e autorização **por evento** (cada tela recebe só o que pode ver).
- **SEC-CLI-6** `postMessage` só com origem verificada; links externos com `rel="noopener noreferrer"`.
- **SEC-CLI-7** Service worker, se existir, não guarda respostas autenticadas.

### B5. Desktop Tauri e periféricos (DESKTOP-MOBILE-AND-LOCAL-IPC)
- **SEC-DSK-1** **Capacidades mínimas** do Tauri: só os comandos necessários a cada janela; nenhum acesso amplo a arquivo, shell ou rede.
- **SEC-DSK-2** O webview carrega **só** a aplicação empacotada ou o servidor VidaCor configurado; conteúdo remoto nunca tem acesso à ponte nativa.
- **SEC-DSK-3** Todo argumento que chega a um comando nativo é validado; o comando não decide permissão: o servidor decide.
- **SEC-DSK-4** Credenciais locais no cofre do sistema operacional; nada em arquivo de texto; limpeza no logout e na troca de usuário.
- **SEC-DSK-5** Atualizador e instalador **assinados**, verificação antes de aplicar, sem *downgrade* para versão vulnerável.
- **SEC-DSK-6** Arquivos temporários (PDF, etiqueta) criados com permissão restrita e apagados; sem corrida entre verificar e abrir (TOCTOU).

### B6. Mensageria, webhooks e protocolos (PROTOCOLS-RPC-AND-MESSAGING)
- **SEC-MSG-1** Cada serviço tem credencial própria no NATS com **permissão só nos assuntos** que precisa (publicar e assinar).
- **SEC-MSG-2** Eventos validados contra schema; valor de enum desconhecido é **rejeitado**, não convertido em padrão.
- **SEC-MSG-3** Identidade do envelope = identidade do conteúdo (o `paciente_id` do evento confere com o do recurso).
- **SEC-MSG-4** Duplicidade, ordem trocada e mensagem velha são tratadas (G3); confirmação (*ack*) só depois de gravar.
- **SEC-MSG-5** Fila de falhas e diagnósticos sem dado de saúde; acesso restrito.
- **SEC-MSG-6** Produtor não confiável (webhook externo) nunca vira comando direto: passa por validação, assinatura e regra de negócio.
- **SEC-MSG-7** DICOM: só AE Titles e IPs cadastrados, validação dos dados do estudo, limites de tamanho por associação.

### B7. Disponibilidade e esgotamento de recursos (RESOURCE-EXHAUSTION-AND-AVAILABILITY)
- **SEC-DIS-1** Limites em tudo: tamanho do corpo, upload, página máxima, profundidade de JSON/XML, tempo por requisição.
- **SEC-DIS-2** Sem processamento de custo superlinear com entrada do usuário (expressões regulares só com o `regex` do Rust, que é linear; nada de *backtracking*).
- **SEC-DIS-3** Descompressão (ZIP de documentos, DICOM comprimido) com limite de tamanho final e de razão de compressão.
- **SEC-DIS-4** Consultas com índice e limite; nenhuma rota faz N consultas por item sem paginação.
- **SEC-DIS-5** **Antes do login, trabalho mínimo:** limite de tentativas no login, na recuperação e no convite.
- **SEC-DIS-6** Pools separados por tipo de trabalho; uma integração lenta não esgota o balcão (G… e A5 do doc 14).
- **SEC-DIS-7** Mensagem envenenada vai para a fila de falhas após N tentativas, sem travar a fila; retentativas com teto e *jitter*.
- **SEC-DIS-8** Tarefa cancelada não continua rodando em segundo plano; arquivos e conexões sempre liberados.

### B8. Isolamento e ciclo de vida dos dados (DATA-ISOLATION-AND-LIFECYCLE)
- **SEC-DAD-1** Campo de dono no registro **não é isolamento**: cada leitura e escrita aplica o filtro (consulta, chave ou RLS).
- **SEC-DAD-2** Cópias derivadas (busca, cache, Indicadores, exportação, prévia, log, réplica, backup) respeitam as **mesmas permissões** da origem.
- **SEC-DAD-3** Logs, métricas e traces **não são leitores alternativos**: nada de dado de saúde neles.
- **SEC-DAD-4** Respostas e contagens não permitem descobrir se um paciente existe (sem oráculo de enumeração).
- **SEC-DAD-5** Exportação e backup com escopo explícito, criptografados e acessíveis só a quem tem permissão; restauração não devolve acessos revogados nem estado inválido.
- **SEC-DAD-6** Links assinados (imagens, PDF) com validade curta e vinculados ao paciente e ao usuário.
- **SEC-DAD-7** Revogação (de consentimento, de acesso, de link compartilhado) vale também para cópias em cache e para trabalhos já enfileirados.
- **SEC-DAD-8** Retenção e exclusão seguem a LGPD e a guarda de 20 anos do prontuário, com regra escrita por tipo de dado.

### B9. Criptografia e segredos (ATTACK-CLASSES + CLOUD-AND-DEPLOYMENT)
- **SEC-CRI-1** Nada de criptografia caseira: bibliotecas consolidadas (`rustls`, `ring`/`aws-lc-rs`); senhas com Keycloak (Argon2/PBKDF2 forte).
- **SEC-CRI-2** Assinatura de webhook comparada em **tempo constante**.
- **SEC-CRI-3** Segredos por serviço, no cofre, com rotação; renovação de certificado monitorada e **falha fechada** (sem cair para conexão sem TLS).
- **SEC-CRI-4** Números aleatórios de segurança (tokens, convites) só de gerador criptográfico.

### B10. Cadeia de suprimentos e publicação (SUPPLY-CHAIN-AND-RELEASE)
- **SEC-SUP-1** Lockfiles versionados; `cargo-deny` com fontes permitidas; sem dependência de registro desconhecido (confusão de nome) **[CI]**.
- **SEC-SUP-2** GitHub Actions fixadas por **SHA**; proibido `pull_request_target` executando código de PR; segredos só em jobs que não rodam código de terceiros **[CI]**.
- **SEC-SUP-3** Permissões do token da CI mínimas por job; cache e artefatos não se misturam entre branches confiáveis e não confiáveis.
- **SEC-SUP-4** Artefato promovido = artefato construído (mesmo hash); assinatura e proveniência por versão.
- **SEC-SUP-5** Metadados de atualização assinados, com proteção contra *rollback*.

### B11. Infraestrutura e implantação (CLOUD-AND-DEPLOYMENT)
- **SEC-INF-1** Contêineres sem root, sistema de arquivos somente leitura, sem *capabilities* extras, sem montar o socket do Docker.
- **SEC-INF-2** Painéis e consoles de administração (Grafana, Orthanc, Keycloak, NATS, banco) **só na VLAN de administração/VPN**, nunca na rede das estações.
- **SEC-INF-3** Serviços internos não acessíveis de onde não precisam (regra por VLAN e por porta).
- **SEC-INF-4** Configuração de segurança com precedência clara: um valor de ambiente não desliga um controle sem aviso.
- **SEC-INF-5** Portal na nuvem: identidade da carga com privilégio mínimo; armazenamento de objetos privado, só por link assinado.

### B12. IA e modelos de linguagem (AI-AND-LLM)
Aplicável ao ditado, ao rascunho de laudo e a qualquer IA futura.
- **SEC-IA-1** Todo conteúdo que entra no modelo (texto do pedido, documento anexado, mensagem de WhatsApp) é **não confiável**: pode conter instruções escondidas (*prompt injection*).
- **SEC-IA-2** A IA **não tem autoridade de escrita**: não assina, não libera, não publica, não envia mensagem; só devolve um rascunho que o médico revisa (R11 do doc 16).
- **SEC-IA-3** Contexto isolado por paciente e por sessão; nada de memória que misture pacientes.
- **SEC-IA-4** Saída da IA renderizada como texto, nunca como HTML ou comando.
- **SEC-IA-5** Dado de saúde só vai para modelo externo com contrato e base legal; preferir modelo local quando possível.
- **SEC-IA-6** Ferramentas e servidores MCP usados por agentes têm escopo mínimo e não herdam a autoridade do usuário sem confirmação.

### B13. Memória, FFI e binários (MEMORY-SAFETY-AND-BINARY)
- **SEC-MEM-1** `unsafe` proibido fora de um crate de FFI dedicado e revisado (ex.: binding da `libxml2`) **[CI]**.
- **SEC-MEM-2** Aritmética de dinheiro e de tamanhos com operações verificadas (`checked_*`), sem estouro silencioso.
- **SEC-MEM-3** *Parsers* (DICOM, XML, retorno de convênio) sem `panic` com entrada malformada, cobertos por *fuzzing* (B4 do doc 14).
- **SEC-MEM-4** Bibliotecas nativas carregadas de caminho fixo e assinado; sem busca em diretório controlado pelo usuário.

---

## Parte C — Como isso entra no dia a dia

1. **No PR:** o autor marca no checklist os domínios tocados (B1–B13) e cita os IDs `SEC-*` atendidos; o revisor usa a skill em **guidance mode** para o trecho.
2. **Na CI:** as regras marcadas **[CI]** viram verificações automáticas desde o esqueleto do projeto.
3. **Nos testes:** cada `SEC-*` relevante a um fluxo crítico tem teste (acesso indevido, CSRF, webhook forjado, upload malicioso, limite de tamanho).
4. **Nas auditorias:** `/security-audit` (full audit) nos momentos da A1; o `/seguranca` e o `/auditoria` dos [playbooks](playbooks/) continuam como roteiros rápidos do G1.
5. **Achado confirmado** → correção mínima + teste de regressão + nova regra `SEC-*`, se a classe ainda não estava coberta.
