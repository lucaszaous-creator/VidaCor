# Segurança de aplicação (AppSec)

Fontes: `docs/17-seguranca-por-classe-de-ataque.md` (Partes A, B e C), `docs/14-regras-de-arquitetura-e-engenharia.md` A9 e B5,
`docs/16-protecao-do-usuario.md` C2, `docs/18-perfis-permissoes-e-rls.md`, ADR-0016, skill `security-audit`.

## 1. Quando aplicar o quê

| Momento | Atividade | Ferramenta |
|---|---|---|
| Módulo novo, integração nova, mudança de fronteira | Modelagem de ameaças STRIDE (docs/16 C2, docs/14 A9) | Modelo do § 6 |
| Todo PR | Domínios `SEC-*` tocados citados; revisão do trecho | Skill `security-audit` em **guidance mode** |
| Todo PR | SAST, dependências, segredos, licenças | CI (`references/ci-cd-e-cadeia-de-suprimentos.md`) |
| Cada versão em `homolog` | DAST | OWASP ZAP contra `homolog`, nunca produção |
| Antes do primeiro uso em produção | Auditoria completa, perfil `deep`; teste de invasão externo | `/security-audit`; consultoria AppSec (docs/25 §4) |
| Antes de cada versão maior | Auditoria completa, perfil `standard` | `/security-audit` |
| Depois de incidente de segurança | Auditoria `scoped` no módulo e fronteiras | `/security-audit` |
| Módulo que toca dinheiro, dado clínico, identidade ou integração | Auditoria `scoped` | `/security-audit` |
| Anualmente | Teste de invasão externo | Consultoria |

Referência de verificação: OWASP ASVS nível 2; **nível 3** para autenticação, auditoria e assinatura
(docs/14 A9).

## 2. Usando a skill `security-audit`

- **Guidance mode** é o padrão: só a parte relevante, sem criar diretório nem rodar as seis fases. Usar
  em revisão de PR e em pergunta pontual.
- **Full audit mode** só quando pedido explicitamente ou nos momentos da tabela acima. Saídas fora do
  repositório (padrão da skill: `~/security-audit-skill/vidacor/run-N`).
- Auditoria é **somente leitura** (G6). Execução de código só em sandbox sem rede externa, com
  ambiente vazio e limites; sem sandbox, o candidato vira `needs_validation` (docs/17 A5).
- Achado `confirmed` vira item rastreado, correção mínima por plano aprovado e teste de regressão. Se a
  classe não estava coberta, nova regra `SEC-*` (docs/17 Parte C).

## 3. Princípios e veredictos (docs/17 A2 a A4)

- Um achado só existe com: quem tem menos confiança, o que envia, qual controle deveria barrar, qual
  fronteira é cruzada, quem é afetado e o **resultado concreto**.
- Controles fora do código (firewall, VLAN, proxy, política do Keycloak) são reais; se faltam no
  repositório, o achado é `needs_validation` com o fato exato que falta.
- Veredictos: `confirmed`, `needs_validation` (sem gravidade), `rejected` (registrado para não reabrir).
- Gravidade só para `confirmed`: crítica, alta, média, baixa, informativa. Pergunta que separa alta de
  média: o resultado **derrota totalmente** um controle explícito com consequência real, ou só o enfraquece?

Anti-padrões proibidos (docs/17 A6): desvio de checklist como vulnerabilidade; "defesa em profundidade"
sem violação alcançável; testar em ambiente real quando a evidência local basta; adivinhar comportamento
de provedor; tratar autoridade legítima como fronteira; exagerar o efeito; resultado só em prosa;
repetir achado antigo; gravidade para `needs_validation`; relatório antes da verificação independente.

## 4. O que conferir na revisão, por domínio

| Domínio | Conferir | IDs |
|---|---|---|
| Injeção | SQL só com `sqlx::query!`; XML sem DTD e entidades externas; Typst com dado escapado; caminho de arquivo gerado pelo servidor; nada via shell | SEC-INJ-1 a 5 |
| Acesso | Rota declara o ato (negar por padrão); autorização **por objeto**; filtro na consulta; teste de acesso indevido em toda rota com ID; RLS forçado com `SET LOCAL` | SEC-ACS-1 a 4, docs/18 §7 |
| Autenticação | OIDC com PKCE, `state`, `nonce`; JWT com algoritmo fixo; sessão renovada e invalidada; cookies `HttpOnly`, `Secure`, `SameSite` e CSRF; 2FA sem atalho; **step-up** para assinar, estornar, autorizar desconto, exportar e mudar permissão; `no-store` em dado de saúde | SEC-AUT-1 a 9 |
| Telas | Sem `dangerouslySetInnerHTML`; CSP estrita com `frame-ancestors 'none'`; nada de token ou dado de saúde no armazenamento do navegador; CORS explícito; WebSocket autorizado por evento | SEC-CLI-1 a 7 |
| Desktop Tauri | Capacidades mínimas; conteúdo remoto sem ponte; comando nativo não decide permissão; credencial no cofre do SO; updater assinado sem *downgrade* | SEC-DSK-1 a 6 |
| Mensageria e DICOM | Permissão por assunto no NATS; enum desconhecido rejeitado; *ack* depois de gravar; webhook nunca vira comando direto; só aparelhos cadastrados | SEC-MSG-1 a 7 |
| Disponibilidade | Limites de corpo, upload, página, profundidade e tempo; regex linear; descompressão limitada; limite de tentativas antes do login; pools separados | SEC-DIS-1 a 8 |
| Dados | Filtro de dono em cada consulta; cópias derivadas com as mesmas permissões; sem oráculo de enumeração; links assinados curtos; revogação vale para cache e fila; retenção por tipo | SEC-DAD-1 a 8 |
| Cripto e segredos | Sem cripto caseira; webhook comparado em tempo constante; segredos por serviço com rotação; falha fechada; aleatório criptográfico | SEC-CRI-1 a 4 |
| Cadeia de suprimentos | Lockfiles, Actions por SHA, permissões mínimas, artefato promovido igual ao construído, atualização assinada | SEC-SUP-1 a 5 |
| Infraestrutura | Contêiner sem root; consoles só na VLAN de administração; serviço só onde precisa; portal com privilégio mínimo | SEC-INF-1 a 5 |
| IA | Entrada não confiável; IA sem autoridade de escrita; contexto por paciente; saída como texto; dado de saúde em modelo externo só com contrato; MCP com escopo mínimo | SEC-IA-1 a 6 |
| Memória | `unsafe` só em crate de FFI revisado; aritmética verificada; parsers sem `panic` e com fuzzing | SEC-MEM-1 a 4 |

## 5. Agentes de IA no desenvolvimento e na operação (SEC-IA-6, docs/14 B6)

- Assistentes seguem o `CLAUDE.md`; auditoria só lê; correção só depois de plano aprovado.
- Agentes e servidores MCP com escopo mínimo: sem credencial de produção, sem acesso ao cofre, sem
  permissão de merge ou de publicação.
- Nenhum dado de paciente, segredo ou código sensível colado em ferramenta externa sem contrato.
- O servidor `e2e mcp` só em `dev` ou `homolog` com dado fictício (docs/28 E2E-18).

## 6. Modelo de ameaças STRIDE de um módulo (template)

```markdown
# Modelo de ameaças: <módulo> (<versão>, <data>)
Autor: <nome>  Revisores: <nomes>  Status: rascunho | revisado

## Escopo e ativos
- Dados tratados e nível (N0 a N4, docs/18 §2):
- Atos sensíveis (step-up, segunda pessoa):
- Integrações e provedores:

## Diagrama de fluxo de dados
<Mermaid com atores, processos, armazenamentos e fronteiras de confiança>

## Ameaças
| ID | Categoria STRIDE | Fronteira | Ameaça (quem, o quê, resultado) | Controle existente | IDs SEC-* | Teste | Situação |
|---|---|---|---|---|---|---|---|
| TM-<MOD>-01 | Falsificação de identidade | Estação para núcleo | ... | OIDC + 2FA | SEC-AUT-1, 5 | ... | Coberto |
| TM-<MOD>-02 | Adulteração | Webhook do PSP | ... | Assinatura no corpo bruto | SEC-CRI-2, G3 | ... | Coberto |
| TM-<MOD>-03 | Repúdio | Ato clínico | ... | Auditoria encadeada | docs/21 | ... | Coberto |
| TM-<MOD>-04 | Divulgação de informação | Logs | ... | Collector remove atributos | SEC-DAD-3 | ... | Coberto |
| TM-<MOD>-05 | Negação de serviço | Upload | ... | Limites | SEC-DIS-1, 3 | ... | Coberto |
| TM-<MOD>-06 | Elevação de privilégio | Rota com ID | ... | Autorização por objeto + RLS | SEC-ACS-2, docs/18 | ... | Coberto |

## Riscos aceitos
<risco, motivo, quem aceitou, data, revisão>

## Pendências
<item, dono, prazo>
```

## 7. Pentest e DAST

- DAST (ZAP) em `homolog` a cada versão, com usuários fictícios de cada perfil.
- Teste de invasão externo antes do primeiro uso em produção e depois anualmente (docs/14 B5).
- Escopo, regras de engajamento e janela por escrito; nunca contra produção com dados reais sem
  autorização formal da clínica, e nunca contra provedores externos.
- Achados entram no mesmo fluxo de achados da auditoria (item rastreado, correção por plano, teste de
  regressão).
