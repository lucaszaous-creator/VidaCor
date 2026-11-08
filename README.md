# VidaCor Imagem — sistema da clínica

> **Tecnologia que cuida da sua vida.**

Este repositório é o projeto **VidaCor**: um sistema próprio para a clínica de diagnóstico por imagem **VidaCor Imagem**. Ele vai substituir os
sistemas que a clínica usa hoje (Medware, Mediworks e Medilab, mais o conversor
para DICOM) e incluir o **estacionamento com leitura de placa**.

## Estado atual

| Etapa | Situação |
|---|---|
| Levantamento dos sistemas atuais (Medware, Medilab) | Feito — [docs/03](docs/03-levantamento-sistemas-atuais.md) |
| Pesquisa do estacionamento com leitura de placa | Feita — [docs/02](docs/02-estacionamento-leitura-de-placa.md) |
| Escopo funcional (todas as funções atuais + novas) | Primeira versão — [docs/04](docs/04-escopo-funcional.md) |
| Design system | Primeira versão — [docs/05](docs/05-design-system.md) |
| Mockups navegáveis (38 telas) | Terceira rodada aprovada — [docs/06](docs/06-mockups.md) |
| Arquitetura e infraestrutura (Rust, on-premises) | Proposta para revisão — [docs/11](docs/11-arquitetura-e-infraestrutura.md) · [ADRs](docs/adr/) |
| Observabilidade e SLOs | Proposta — [docs/12](docs/12-observabilidade-e-metricas.md) |
| Instalação e distribuição | Proposta — [docs/13](docs/13-instalacao-e-distribuicao.md) |
| Regras de arquitetura e engenharia | Proposta — [docs/14](docs/14-regras-de-arquitetura-e-engenharia.md) |
| Padrão de qualidade (seis garantias + playbooks) | Proposta — [CLAUDE.md](CLAUDE.md) · [docs/15](docs/15-padrao-de-qualidade.md) · [playbooks](docs/playbooks/) |
| Proteção do usuário (processo + interface) | Regra — [docs/16](docs/16-protecao-do-usuario.md) · [modelos](docs/modelos/) |
| Segurança por classe de ataque (Cloudflare) | Regra — [docs/17](docs/17-seguranca-por-classe-de-ataque.md) · `/security-audit` |
| Testes ponta a ponta das telas (skill e2e) | Regra — [docs/28](docs/28-testes-ponta-a-ponta-e2e.md) · skill `e2e` |
| Time de desenvolvimento (uma skill por papel) | Regra — [CLAUDE.md](CLAUDE.md) · skill `time-vidacor` |
| Perfis, permissões e RLS | Proposta para validar — [docs/18](docs/18-perfis-permissoes-e-rls.md) |
| Lastro e rastreabilidade | Regra — [docs/21](docs/21-lastro-e-rastreabilidade.md) |
| Lacunas e roteiro (essencial, importante, futuro) | Aprovado — [docs/22](docs/22-lacunas-e-roadmap.md) |
| Desenvolvimento | ⏳ Próximo passo — [docs/09](docs/09-proximos-passos.md) |

**Mockup ao vivo:** https://claude.ai/artifact/NT2bm1UnVPJe9NGGs3HTii
(privado; compartilhe pelo menu *Share* da página para outras pessoas abrirem).

## Exames realizados pela clínica

Ecocardiograma · Doppler de carótidas · USGs obstétricas e diversas (mais de 50
tipos) · Teste ergométrico · Raio X · Tomografia · MAPA. A fachada também cita
ressonância, mamografia e medicina nuclear.

## O que tem aqui

```
README.md                    este arquivo
CLAUDE.md / AGENTS.md        regras obrigatórias (as seis garantias) para pessoas e IA
.claude/commands/            playbooks como comandos (/auditoria, /idempotencia…)
.claude/skills/security-audit/  skill de auditoria da Cloudflare (MIT)
.claude/skills/e2e/          skill de testes ponta a ponta da TesterArmy (Apache-2.0)
.claude/skills/time-vidacor/ coordenação do time: papéis, portões, RACI, passagens, arranque da Fase 0
.claude/skills/produto/ ux-ui/ desenvolvimento/ qa/ devops-seguranca/  uma skill por papel
THIRD_PARTY_NOTICES.md       licenças de terceiros
docs/
  01-historico-da-conversa.md   tudo o que foi discutido e decidido até agora
  02-estacionamento-leitura-de-placa.md
  03-levantamento-sistemas-atuais.md
  04-escopo-funcional.md        módulos × funções, cobertura Medware/Medilab
  05-design-system.md           cores, tipografia, componentes, regras de UX
  06-mockups.md                 as 25 telas, rodadas de feedback e decisões
  07-arquitetura-proposta.md    (substituída pela 11)
  08-seguranca-lgpd-etica.md
  09-proximos-passos.md
  10-assets-e-licencas.md       origem do modelo 3D, imagens DICOM e logo
  11-arquitetura-e-infraestrutura.md  núcleo em Rust, poliglota, servidor local
  12-observabilidade-e-metricas.md    percentis, RED/USE, Web Vitals, SLOs
  13-instalacao-e-distribuicao.md     instaladores do servidor e das estações
  14-regras-de-arquitetura-e-engenharia.md  princípios, padrões, testes, segurança, fluxo
  15-padrao-de-qualidade.md     as seis garantias com exemplos do VidaCor e checklist de PR
  16-protecao-do-usuario.md     a cerca (processo) e a rede de segurança (interface)
  17-seguranca-por-classe-de-ataque.md  regras SEC-* baseadas na skill Cloudflare security-audit
  18-perfis-permissoes-e-rls.md quem vê e faz o quê; RLS no PostgreSQL
  19-menu-de-contexto.md        clique direito: regras e catálogo por objeto
  20-etiquetas-e-rastreio.md    tipos de etiqueta, o que imprimir e o lastro
  21-lastro-e-rastreabilidade.md  metadados, cadeia do atendimento, guia obrigatória, 16 domínios
  22-lacunas-e-roadmap.md       o que faltava para clínicas grandes, priorizado
  23-modo-contingencia.md       quando o sistema cai: kit das 06:00, papel numerado, retomada e conciliação
  24-plano-de-migracao.md       Medware, Mediworks, Medilab e conversor: inventário, ondas, conferência, virada
  25-plano-do-projeto.md        MVP, fases, equipe, cronograma, orçamento, contrato e governança (premissas a confirmar)
  26-teste-de-usabilidade.md    roteiro do teste com a equipe, tarefas críticas por perfil, métricas
  27-frases-de-comportamento.md frases Dado/Quando/Então dos fluxos críticos, para aprovação antes dos testes
  28-testes-ponta-a-ponta-e2e.md  regras E2E-* para testes de tela com a skill e2e
  modelos/                      design, riscos clínicos, rastreabilidade, prontidão, pós-incidente, pacote do responsável clínico
  playbooks/                    auditoria, testes, refatoração, segurança, idempotência, corrida
  bugs.md                       bugs encontrados por testes
  adr/                          decisões de arquitetura (ADR-0001 a 0013)
design/
  mockups/                   fonte das telas (.dc.html) + canvas.json
  assets/vidacor.css         design system usado pelos mockups
  assets/marca/              logo VidaCor (recortes) e imagem de referência
  assets/anatomia/           renders anatômicos realistas (BodyParts3D)
  assets/imagens-exame/      imagens ilustrativas de exame (DICOM de exemplo)
  referencias/               referências de UI e prints de feedback do cliente
tools/
  render-anatomia/           como o mapa corporal 3D foi renderizado
  imagens-dicom/             como as imagens de exame foram extraídas
```

## Princípios do produto

- **Uma clínica, um sistema:** agenda, recepção, caixa, atendimento, exames,
  laudos, imagens, faturamento TISS, financeiro, estoque, WhatsApp, portal e
  estacionamento no mesmo lugar.
- **Roda no servidor da clínica**, funciona sem internet (exceto PIX/WhatsApp),
  com backup criptografado fora do local.
- **Registro clínico nunca é apagado**: correção vira nova versão com motivo.
- **Acessível e rápido no balcão**: contraste AA, status com cor + texto,
  teclado, telas que não espremem.
- **LGPD de verdade**: consentimento por finalidade, auditoria de acesso,
  portal do paciente seguro.
