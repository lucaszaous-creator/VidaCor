# 14 · Regras de arquitetura e de engenharia

> **Status:** proposta (v0.1, nov/2026). Consolida as regras que valem para
> todo o código do VidaCor. Complementa a [11](11-arquitetura-e-infraestrutura.md)
> (arquitetura), a [12](12-observabilidade-e-metricas.md) (métricas) e a
> [13](13-instalacao-e-distribuicao.md) (instalação). A **Parte 0** traz as seis garantias obrigatórias; os exemplos completos e o
> checklist de PR estão em [15](15-padrao-de-qualidade.md) e o resumo no
> [CLAUDE.md](../CLAUDE.md). Regras marcadas com
> **[CI]** são verificadas automaticamente: o merge não acontece se falharem.

---

## Parte 0 — As seis garantias (obrigatórias, acima de qualquer outra regra)

| # | Garantia | Regra | Playbook |
|---|---|---|---|
| **G1** | **O servidor decide; o cliente só pede** | Preço, desconto, cortesia, valor de guia, permissão, perfil e limite são calculados e validados no núcleo; valores vindos da tela são ignorados e recalculados. Nenhum segredo em código de tela ou variável do navegador (`VITE_*`, Tauri). Banco nunca exposto diretamente; RLS por paciente no portal | [/seguranca](playbooks/seguranca.md) |
| **G2** | **Concorrência se resolve no banco** | Todo "ler, decidir, gravar" sobre recurso limitado é protegido no PostgreSQL, nesta ordem: constraint única → operação atômica (`UPDATE … WHERE condição`, conferindo linhas) → `SELECT … FOR UPDATE`. Proibido lock em memória, Redis com expiração ou cache como proteção (só com ADR). Teste com 2 requisições simultâneas | [/condicao-de-corrida](playbooks/condicao-de-corrida.md) |
| **G3** | **Repetir não duplica** | Webhooks: assinatura verificada no corpo bruto, ID do evento gravado com chave única **antes** de agir, independência da ordem, 2xx rápido + outbox. Chamadas a provedores com chave de idempotência por operação. Rotas de escrita aceitam `Idempotency-Key`. Nunca dado pessoal como chave | [/idempotencia](playbooks/idempotencia.md) |
| **G4** | **Teste de integração de verdade** | Rota HTTP real, PostgreSQL real de teste, mock só de terceiros. Cobre caminho feliz, erros, repetição, limites e acesso indevido. Frases aprovadas pelo dono antes. Proibido apagar, pular ou afrouxar teste; bug real fica falhando e vai para [bugs.md](bugs.md). Comando único `just test` | [/testes-integracao](playbooks/testes-integracao.md) |
| **G5** | **Camadas separadas** | Tela sem regra; domínio sem I/O; adaptadores na borda. 400 linhas = revisão, 600 = justificar ou dividir. Refatoração sem mudar comportamento, com testes de caracterização | [/refatoracao-segura](playbooks/refatoracao-segura.md) |
| **G6** | **Plano antes, alteração depois** | Auditoria só lê. Correção começa por plano e espera o OK do dono. Migration revisada por pessoa; nada apagado sozinho; nenhuma proteção desligada para "fazer funcionar". O que não deu para verificar é escrito como "não verificado" | [/auditoria](playbooks/auditoria.md) |

Além delas, valem os procedimentos e requisitos de interface de **proteção do usuário** do [docs/16](16-protecao-do-usuario.md) (a cerca e a rede de segurança), e as regras de construção por classe de ataque `SEC-*` do [docs/17](17-seguranca-por-classe-de-ataque.md) (baseadas na skill Cloudflare security-audit) e as regras de teste de tela `E2E-*` do [docs/28](28-testes-ponta-a-ponta-e2e.md) (skill e2e da TesterArmy).

As Partes A e B abaixo detalham **como** cumprir essas garantias e o resto da engenharia.

---

## Parte A — Arquitetura

### A1. Princípios de design
| Regra | O que significa no VidaCor |
|---|---|
| **DDD (Domain-Driven Design)** | Código fala a língua da clínica: `Guia`, `Laudo`, `Encaixe`, `Glosa`. Um glossário (linguagem ubíqua) fica em `docs/glossario.md`. Cada módulo é um *bounded context* com seus agregados e invariantes |
| **Arquitetura hexagonal (portas e adaptadores)** | O domínio não conhece banco, HTTP, Orthanc ou WhatsApp. Ele define *traits* (portas); os adaptadores ficam na borda. Assim é possível testar a regra de negócio sem infraestrutura |
| **SOLID e baixo acoplamento** | Módulos dependem de abstrações; nenhum módulo lê tabela de outro **[CI: teste de arquitetura]** |
| **CQRS onde ajuda** | Escrita pelo modelo de domínio; leitura pesada (Indicadores, Início, listas) por *read models* e views materializadas |
| **Registro clínico imutável** | Laudo, evolução e documento assinado nunca são alterados nem apagados: correção gera nova versão com motivo (Lei 13.787/2018). Proibido `DELETE`/`UPDATE` nessas tabelas **[CI: lint de migração]** |
| **12-Factor App** | Configuração por ambiente, processos sem estado, logs como fluxo, paridade dev/homolog/produção |
| **Simplicidade primeiro** | Não introduzir componente novo (cache, fila, serviço) sem medição que justifique e sem ADR |

### A2. Consistência e integridade de dados
- **Transações curtas**; nada de chamada externa dentro de transação de banco.
- **Outbox transacional** para todo evento que sai do núcleo.
- **Sagas** (passos com compensação) para fluxos longos, como check-in → guia → cobrança → exame.
- **Idempotência** em toda operação que pode ser repetida: chave `Idempotency-Key` nas APIs de escrita e deduplicação nos consumidores (**G3**).
- **Recursos limitados** (horário de sala, senha, caixa, lote de estoque, número de guia, convite, versão de laudo) protegidos no banco por constraint única, operação atômica ou `FOR UPDATE`, nunca por lock em memória (**G2**).
- **Concorrência otimista** (coluna de versão) em agregados editados por mais de uma pessoa (agenda, caixa).
- Restrições no banco (FK, `CHECK`, `UNIQUE`, `NOT NULL`) reforçam as invariantes do domínio. O banco nunca depende só do código.

### A3. Tipos e valores que costumam gerar bugs
| Tipo | Regra |
|---|---|
| **Dinheiro** | Inteiro em **centavos** (`i64`) ou `Decimal`; nunca `f32/f64`. Arredondamento explícito e documentado |
| **Data e hora** | Gravar em **UTC** (`timestamptz`), exibir em `America/Sao_Paulo`. Datas sem hora (nascimento) como `date` |
| **Identificadores** | **UUIDv7** (ordenável por tempo) como chave; número de exame (*accession*) e protocolo legíveis à parte |
| **Documentos BR** | Tipos próprios validados (`Cpf`, `Cnpj`, `Cns`, `Placa`, `CodigoTuss`, `Cid10`), nunca `String` solta |
| **Texto** | UTF-8 e normalização Unicode na busca (acentos); sem ordenação dependente da máquina |
| **Unidades clínicas** | Valores com unidade (`mm`, `%`, `mmHg`), nunca número solto |

### A4. Padrões de API
- REST com recursos no plural, verbos HTTP corretos, **versão no caminho** (`/v1`).
- Erros no formato **Problem Details (RFC 9457)**, com código de erro estável e mensagem em pt-BR para a tela.
- Paginação por cursor; filtros e ordenação explícitos; limites máximos.
- **OpenAPI gerado do código** é o contrato; mudança incompatível só em nova versão **[CI: diff de contrato]**.
- Tempo real por **WebSocket/SSE** com eventos tipados e reconexão com retomada.
- Toda resposta carrega `trace_id` para suporte.

### A5. Resiliência
- **Timeout** em toda chamada de rede (nenhuma espera infinita).
- **Retentativa com backoff exponencial e jitter**, apenas para operações idempotentes.
- **Circuit breaker** por integração externa; **bulkhead** (pools separados) para que uma integração lenta não derrube o balcão.
- **Rate limiting** e **backpressure** nas filas.
- **Degradação graciosa**: sem internet, o atendimento continua; PIX, WhatsApp e NFS-e ficam pendentes e são reprocessados.
- *Health checks* de prontidão e vivacidade em todo serviço.

### A6. Evolução sem quebrar produção
- **Migrações em duas etapas (expand/contract)**: primeiro adicionar, migrar dados e código, depois remover em outra versão.
- **Feature flags** para liberar funções aos poucos (por perfil, máquina ou unidade).
- Compatibilidade de versão entre Desktop e servidor por um período definido.
- Eventos com campo `versao`; consumidores toleram campos novos.

### A7. Padrões de saúde e interoperabilidade
| Padrão | Uso |
|---|---|
| **DICOM** (MWL, MPPS, C-STORE, SR, DICOMweb) | Aparelhos e PACS |
| **TISS 4.01 / TUSS** (ANS) | Faturamento de convênios |
| **CID-10** (preparado para CID-11) | Diagnósticos |
| **HL7 FHIR R4** | Interoperabilidade externa e futura integração com a **RNDS** (Rede Nacional de Dados em Saúde) |
| **ICP-Brasil** (PAdES, CAdES, XAdES) | Assinatura de documentos |
| **NFS-e Nacional** | Notas de serviço |

### A8. Conformidade regulatória (requisitos de arquitetura)
- **LGPD**: privacidade desde a concepção, minimização de dados, base legal por finalidade, registro de operações de tratamento.
- **Lei 13.787/2018** e **Resolução CFM 1.821/2007**: prontuário eletrônico, guarda de 20 anos, assinatura digital.
- **Certificação SBIS/CFM (S-RES)**: usar os requisitos do manual como referência de segurança e de prontuário, mesmo antes de buscar o selo.
- **Anvisa RDC 657/2022**: avaliar se algum componente (visualizador diagnóstico, IA) se enquadra como software dispositivo médico.
- **ANS**: padrão TISS vigente.

### A9. Segurança na arquitetura
- **Modelagem de ameaças (STRIDE)** para cada módulo novo e cada integração.
- Referência de verificação: **OWASP ASVS nível 2** (nível 3 para autenticação, auditoria e assinatura).
- **Menor privilégio** em tudo: usuários, serviços, banco (um papel por módulo), rede (VLANs).
- **Confiança zero na rede interna**: TLS e autenticação também entre serviços locais.
- **Regra no servidor** (**G1**): nada de preço, desconto, permissão ou limite decidido só na tela.
- **Classificação de dados**: público · interno · pessoal · **saúde (sensível)**. Dado de saúde nunca em log, métrica, URL ou cache do navegador.
- Segredos só no cofre (SOPS/age); nunca no código ou em imagem **[CI: varredura de segredos]**.

---

## Parte B — Desenvolvimento

### B1. Fluxo de trabalho
- **Trunk-based** com branches curtas (≤ 2 dias); `main` sempre implantável **[CI: branch protegida]**.
- **Pull requests pequenos** (alvo ≤ 400 linhas alteradas) com descrição, evidência (teste e print) e vínculo à tarefa.
- **Revisão obrigatória** por pelo menos 1 pessoa; **2 pessoas** em módulos críticos (laudos, assinatura, faturamento, caixa, auditoria, identidade) via `CODEOWNERS`.
- **Conventional Commits** e **SemVer**; changelog e notas de versão gerados.
- **RFC** (documento curto de proposta) antes de funcionalidades grandes; **ADR** para decisões estruturais.

### B2. Definition of Ready e Definition of Done
**Pronto para começar:** problema e critério de aceite claros, tela no mockup ou esboço, regras de negócio validadas com a clínica, impacto em LGPD avaliado.

**Pronto (done):**
1. Código revisado e mesclado; testes automatizados passando **[CI]**.
2. Critérios de aceite verificados em homologação.
3. Acessibilidade verificada (teclado, leitor de tela, contraste) **[CI: axe]**.
4. Métricas, logs e traces instrumentados; orçamento de latência respeitado **[CI: k6 nas rotas críticas]**.
5. Documentação atualizada (OpenAPI, ADR/RFC, manual do usuário se mudou tela).
6. Migração reversível ou plano de contorno documentado.
7. *Feature flag* definida, quando aplicável.
8. Checklist das seis garantias (G1–G6) do [docs/15](15-padrao-de-qualidade.md#checklist-do-pull-request) marcado no PR.

### B3. Regras de código por linguagem
| Linguagem | Regras |
|---|---|
| **Rust** | `rustfmt` e `clippy -D warnings` **[CI]**; proibido `unwrap()`/`expect()`/`panic!` em código de produção (só em testes) **[CI: clippy]**; erros com `thiserror` no domínio e `anyhow` só na borda; `#![forbid(unsafe_code)]` no domínio; funções assíncronas com cancelamento seguro; sem `clone()` para contornar o *borrow checker* sem justificativa |
| **TypeScript** | `strict`, proibido `any` **[CI]**; dados do servidor só pelo cliente gerado; estado do servidor no TanStack Query; componentes acessíveis do design system em vez de HTML improvisado |
| **Python** | `ruff` e `mypy --strict` **[CI]**; dependências travadas; workers sem estado |
| **Kotlin** | `ktlint` e `detekt` **[CI]**; sem `!!`; nulabilidade explícita |
| **SQL** | Nomes em `snake_case` e em português do domínio; toda FK indexada; sem `SELECT *` no código; migrações revisadas **[CI: squawk]** |

Gerais: nomes descritivos no idioma do domínio (pt-BR para o negócio, inglês para termos técnicos consagrados); funções curtas e com uma responsabilidade; comentários explicam o **porquê**, não o quê; nenhum código morto ou comentado.

### B4. Testes
| Tipo | Regra |
|---|---|
| **Estilo request spec (G4)** | Fluxo crítico testado na rota HTTP real com banco real; mock só de terceiros; casos feliz, erro, repetição, limite e acesso indevido; proibido pular ou afrouxar teste |
| **Pirâmide** | Muitos testes de unidade (domínio), testes de integração com Testcontainers (PostgreSQL, Orthanc, NATS) e poucos testes ponta a ponta (Playwright) nas jornadas principais |
| **Cobertura** | Mínimo de **80%** nos crates de domínio e **90%** em laudos, faturamento, caixa e assinatura **[CI]** |
| **Testes de mutação** | `cargo-mutants` nos módulos críticos, para provar que os testes pegam erro de verdade |
| **Testes de propriedade** | `proptest` para regras com muitas combinações (convênios, cortesia, cálculo de caixa) |
| ***Fuzzing*** | `cargo-fuzz` nos *parsers* (DICOM, XML TISS, retorno de convênio, leitura de placa) |
| **Contrato** | Diff do OpenAPI e dos schemas de evento a cada PR **[CI]** |
| **Regressão visual** | Prints das telas e dos PDFs (laudo, recibo, etiqueta) comparados a cada versão |
| **Desempenho** | `k6` com orçamento p95/p99 por rota **[CI nas rotas críticas]** |
| **Dados de teste** | Sintéticos ou anonimizados; **proibido** dado real de paciente fora de produção |
| **Testes instáveis** | Teste intermitente é tratado como bug e corrigido ou isolado no mesmo dia |

### B5. Segurança no ciclo de desenvolvimento
- **SAST** (Semgrep, CodeQL), **varredura de dependências** (`cargo-audit`, `cargo-deny`, `pnpm audit`, `pip-audit`) e **varredura de segredos** a cada PR **[CI]**.
- **DAST** (OWASP ZAP) em homologação a cada versão.
- **Conformidade de licenças** (`cargo-deny licenses`): bloquear licenças incompatíveis com uso comercial e registrar atribuições (por exemplo, BodyParts3D CC BY-SA) **[CI]**.
- **SBOM** e imagens assinadas por versão; builds reproduzíveis.
- Teste de invasão externo antes do primeiro uso em produção e depois anualmente.

### B6. Uso de assistentes de IA no código
- Permitido como ferramenta, **com as mesmas regras de qualquer código**: revisão humana, testes, lint e cobertura.
- Quem abre o PR é responsável pelo código, inclusive pelo que foi gerado.
- Assistentes seguem o [CLAUDE.md](../CLAUDE.md) e os [playbooks](playbooks/): auditoria só lê, correção só depois de plano aprovado (**G6**).
- Proibido colar dados de pacientes, segredos ou código sensível em ferramentas externas sem contrato que garanta a privacidade.

### B7. Documentação como código
- Documentação vive no repositório, revisada no mesmo PR da mudança.
- Diagramas no modelo **C4** (contexto, contêineres, componentes) em Mermaid.
- **Runbooks** por serviço: como reiniciar, restaurar, diagnosticar.
- Manual do usuário por módulo e guia de instalação para a TI da clínica.

### B8. Operação, incidentes e melhoria contínua
- **Severidades de incidente** (S1: atendimento parado; S2: módulo indisponível; S3: degradação; S4: incômodo) com tempo de resposta definido.
- **Post-mortem sem culpados** para S1 e S2, com ações rastreadas.
- **Exercícios periódicos**: restauração de backup (mensal), *failover* para o servidor de contingência (trimestral), resposta a incidente de segurança (semestral).
- Indicadores **DORA** e SLOs revisados mensalmente.

### B9. Acessibilidade e UX como regra
- **WCAG 2.2 nível AA** como requisito, não como melhoria.
- Toda tela utilizável só pelo teclado, com foco visível e atalhos documentados.
- Textos e mensagens em pt-BR claro, com o vocabulário da clínica; erros dizem **o que fazer**.
- Regras de layout do [05 · Design system](05-design-system.md) (nada espremido, uma próxima ação, status com cor + texto).
