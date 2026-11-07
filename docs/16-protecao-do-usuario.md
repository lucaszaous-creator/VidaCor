# 16 · Proteção do usuário: a cerca e a rede de segurança

> **Status:** regra obrigatória (v0.1, nov/2026). Decisão no
> [ADR-0015](adr/0015-protecao-do-usuario.md). Resumo no [CLAUDE.md](../CLAUDE.md).
> Complementa as seis garantias técnicas (G1–G6) do [docs/15](15-padrao-de-qualidade.md).

**Objetivo:** nenhum erro chega à recepcionista, ao técnico, ao médico, à gerência
ou ao paciente. Para isso há duas camadas:

1. **A cerca** (regras internas da equipe, invisíveis ao usuário): impede que o erro saia da engenharia.
2. **A rede de segurança** (regras de experiência do usuário): se algo passar, o usuário não sofre o erro nem é levado a cometer um.

Nenhum código, tela ou lançamento pode ultrapassar estes caminhos. Quando uma
regra não puder ser cumprida, a exceção é escrita no PR, aprovada pelo dono do
produto e, se for clínica, pelo **responsável clínico de segurança** (§ C1).

---

## Parte 1 — A cerca (antes de chegar ao usuário)

### C1. Papéis
| Papel | Responsabilidade |
|---|---|
| **Dono do produto** | Fonte da verdade sobre o comportamento esperado; aprova frases de teste, planos e exceções |
| **Responsável clínico de segurança** | Médico da VidaCor que aprova toda mudança em fluxo clínico (identificação, exame, laudo, prescrição, portal) e o registro de riscos |
| **Dono técnico do módulo** | Responde pelo módulo em produção ("quem constrói, opera") |
| **Revisores (CODEOWNERS)** | Duas pessoas nos módulos críticos: laudos, assinatura, faturamento, caixa, auditoria, identidade, exames/DICOM |

### C2. Antes de construir
1. **Documento de design** (modelo: [modelos/documento-de-design.md](modelos/documento-de-design.md)) para todo módulo ou função grande. Começa pelo usuário ("trabalhar de trás para frente"): qual problema real, para quem, como ele saberá que funcionou.
2. **Análise de risco clínico** (ISO 14971, versão enxuta): a mudança entra no [registro de riscos](modelos/registro-de-riscos-clinicos.md) com perigo, causa, gravidade, probabilidade e controle. **Risco alto sem controle não é construído.**
3. **Modelagem de ameaças** (STRIDE) quando toca dado pessoal, dinheiro, integração ou permissão.
4. **Requisitos rastreáveis** (IEC 62304, versão enxuta): todo requisito tem ID (`REQ-AGD-012`) e aponta para o teste que o comprova ([modelo](modelos/rastreabilidade.md)).
5. **Tarefas críticas de usabilidade** identificadas: as que, feitas errado, causam dano (identificar paciente, laudar, prescrever, cobrar, publicar resultado).

### C3. Ao construir
1. As seis garantias **G1–G6** do [docs/15](15-padrao-de-qualidade.md).
2. **Perfis IHE** na integração com aparelhos: **SWF** (pedido → worklist → MPPS → imagem) e **PIR** (correção de identificação nas imagens). Integração fora do perfil exige ADR.
3. **DICOM Conformance Statement**: o VidaCor publica o seu, e nenhum aparelho é integrado sem o do fabricante.
4. **Dois identificadores do paciente** (nome completo + data de nascimento) em todo ato clínico: check-in, sala de exame, conciliação de imagem, laudo, prescrição e entrega de resultado. O sistema bloqueia o ato sem a confirmação.
5. **Regras de interface da Parte 2** são requisitos de código, não sugestão.
6. **Segurança desde a concepção** (SDL): requisito → design → código → teste → lançamento → resposta, com a verificação de cada fase registrada no PR.

### C4. Ao testar
1. Testes da G4 (rota real, banco real, repetição, limites, acesso indevido).
2. **Teste de usabilidade com a equipe da clínica** (IEC 62366) nas tarefas críticas, antes de cada módulo entrar em produção. Critério: a tarefa é feita sem erro e sem ajuda por pelo menos 4 de 5 participantes; erro em tarefa crítica bloqueia o lançamento.
3. **Engenharia do caos leve** em homologação, a cada versão maior: derrubar de propósito banco, PACS, NATS, internet e um provedor externo, e provar que o atendimento segue e nada se perde.
4. **Teste de acessibilidade** automático (axe) e manual (teclado e leitor de tela) nas telas novas.

### C5. Ao lançar
1. **Revisão de prontidão para produção** (modelo: [modelos/prontidao-para-producao.md](modelos/prontidao-para-producao.md)): monitoramento, alertas, manual de operação, capacidade, plano de volta, treinamento feito. Sem checklist completo, não vai para produção.
2. **Uso interno primeiro** (*dogfooding*): a equipe da clínica usa em homologação com dados fictícios.
3. **Canal piloto** (uma ou duas máquinas) com **comparação automática** de erros e latência contra a versão atual (análise canário). Piora → volta sozinha.
4. **Liberação gradual por chave liga/desliga**: por perfil, máquina ou sala, nunca a clínica inteira de uma vez.
5. **Janela de lançamento** fora do horário de pico, com o dono técnico disponível e a volta à versão anterior testada.
6. **Congelamento**: nenhum lançamento em véspera de feriado, fechamento de mês do faturamento ou durante incidente aberto.

### C6. Ao operar
1. SLOs, orçamento de erro e alertas do [docs/12](12-observabilidade-e-metricas.md). **Orçamento estourado = só correções até normalizar.**
2. Exercícios: restauração de backup (mensal), troca para o servidor de contingência (trimestral), incidente de segurança (semestral).
3. **Redução de trabalho manual repetitivo**: tarefa operacional feita à mão mais de 3 vezes vira automação ou manual de operação.

### C7. Depois de um problema
1. **Análise pós-incidente sem culpados** (modelo: [modelos/pos-incidente.md](modelos/pos-incidente.md)) para S1 e S2 e para **todo incidente clínico** (identificação errada, resultado errado ou no lugar errado), com "5 porquês", ações com dono e prazo.
2. Toda ação vira **teste, regra no CLAUDE.md, verificação na CI ou risco no registro**, para que o mesmo erro não volte.
3. Incidente clínico é comunicado ao responsável clínico de segurança **no mesmo dia**.

---

## Parte 2 — A rede de segurança (o que o usuário vê)

Cada item é um **requisito de interface**, verificado na revisão de código e nos testes de usabilidade.

### R1. Prevenir em vez de avisar
- O sistema **não deixa** o erro acontecer: horário ocupado não aparece como livre; laudo não abre para estudo sem paciente conciliado; prescrição de medicamento com alergia registrada é bloqueada até justificativa do médico; caixa não recebe sem caixa aberto.
- Campos aceitam só o formato válido (CPF, data, placa, valor) e explicam o formato antes do erro.

### R2. Dois identificadores, sempre visíveis
- Nome completo e data de nascimento (e foto, quando houver) aparecem **no topo de toda tela clínica** e em todo documento impresso.
- Pacientes com nome parecido no mesmo dia recebem **alerta de homônimo**.

### R3. Confirmação só onde importa
- Confirmação explícita apenas para atos irreversíveis ou de alto impacto: assinar e liberar laudo, estornar, cancelar exame, publicar resultado, fechar caixa, vincular imagem manualmente.
- A confirmação diz **o que** vai acontecer e **com quem** ("Assinar o laudo de **Sônia Maria Castro**, 03/02/1962, e liberar no portal?"), nunca "Tem certeza?".

### R4. Política de alertas (contra a fadiga de alertas)
| Nível | Quando | Como aparece |
|---|---|---|
| **Bloqueante** | Risco ao paciente ou perda de dinheiro | Impede o ato até resolver ou justificar |
| **Atenção** | Precisa de ação hoje | Etiqueta na tela, sem interromper |
| **Informativo** | Contexto útil | Texto discreto |
Novo alerta bloqueante exige aprovação do responsável clínico de segurança.

### R5. Desfazer e corrigir, nunca apagar
- Ação comum tem **desfazer** por alguns segundos (mover agendamento, chamar senha errada).
- Registro clínico e financeiro se corrige com **nova versão + motivo**; o histórico continua visível para quem tem permissão.

### R6. Nada se perde
- Rascunho salvo automaticamente em laudo, evolução, cadastro e recebimento.
- Queda de rede: a tela avisa, mantém o que foi digitado e reenvia ao reconectar, sem duplicar (G3).

### R7. O atendimento nunca para
- Sem internet, convênio, PIX, WhatsApp ou NFS-e, o atendimento continua; o que depende de fora entra numa **lista de pendências** visível, com quem resolve e quando.
- Falha de um módulo não derruba os outros.

### R8. Mensagens de erro que ajudam
- Em português da clínica, sem código técnico na frente do usuário.
- Formato: **o que aconteceu → o que já foi feito → o que fazer agora**.
  Exemplo: "O convênio não respondeu à consulta de elegibilidade. O check-in foi concluído e a consulta será refeita automaticamente. Se precisar, ligue para a central do convênio."
- A tela **nunca mente**: falha não aparece como sucesso, ausência de dado não aparece como zero.
- Todo erro carrega um código de suporte (o `trace_id`) para a TI localizar o caso.

### R9. Consistência e previsibilidade
- Mesma ação, mesmo lugar, mesmo nome em todas as telas (design system do [docs/05](05-design-system.md)).
- Mudança visível de tela ou de fluxo é **avisada antes** à equipe, com o que mudou e por quê.

### R10. Acessibilidade e conforto
- WCAG 2.2 AA, uso completo pelo teclado, foco visível, contraste, telas que não espremem, letra ajustável no portal e no app.

### R11. Ética com o paciente
- Resultado delicado **não vai direto** ao portal nem ao "Meus Exames Bebê": o médico marca "conversar antes" e libera depois.
- Uso de IA é **transparente** e sempre revisado: rascunho de IA aparece marcado e só vira laudo depois de editado ou aprovado pelo médico.
- O paciente vê quem acessou seus dados (resumo da auditoria no portal), baixa os próprios dados e controla os consentimentos.
- Nenhum padrão enganoso: nada de opção pré-marcada para marketing, nada de dificultar o cancelamento de consentimento.

### R12. Treinamento e prática segura
- **Ambiente de treinamento** com dados fictícios, igual ao de produção.
- Manual por módulo e vídeos curtos das tarefas críticas.
- Nova função crítica só é liberada para quem fez o treinamento (por perfil).

### R13. Comunicação honesta em incidentes
- Quando algo falha, a clínica é avisada pelo painel e pelo canal de suporte: **o que aconteceu, o impacto, o que fazer enquanto isso e a previsão de volta**.
- Depois, um resumo da causa e do que foi feito para não repetir.

### R14. Suporte com dono
| Gravidade | Exemplo | Primeira resposta |
|---|---|---|
| S1 | Atendimento parado, risco ao paciente | 15 min |
| S2 | Módulo indisponível | 1 h |
| S3 | Lentidão, contorno disponível | 1 dia útil |
| S4 | Incômodo, sugestão | 5 dias úteis |
Quem reportou sempre recebe retorno do desfecho.

---

## Parte 3 — Referências que inspiraram estas regras

| Fonte | O que adotamos |
|---|---|
| Google SRE | SLO, orçamento de erro, pós-incidente sem culpados, revisão de prontidão, redução de trabalho manual |
| Amazon | Trabalhar de trás para frente (documento de design centrado no usuário), "5 porquês", quem constrói opera |
| Netflix | Engenharia do caos, análise canário, caminho pavimentado |
| Meta | Chaves liga/desliga, lançamento gradual, uso interno primeiro, níveis de incidente |
| Microsoft SDL | Segurança em todas as fases |
| Guias públicos do Claude Code (Anthropic) | Contexto no CLAUDE.md, planejar antes de editar, testes, revisão humana do que a IA gera |
| IEC 62304 · ISO 14971 · IEC 62366-1 | Rastreabilidade, gestão de risco clínico, usabilidade com usuários reais |
| IHE SWF / PIR · DICOM Conformance | Integração com aparelhos e correção de identificação |
| Metas internacionais de segurança do paciente | Dois identificadores |
| Estilo 21 CFR Part 11 | Assinatura vinculada ao registro, trilha inalterável |
| DCB0129 (NHS) | Responsável clínico de segurança |
| LGPD · Lei 13.787 · CFM 1.821 · SBIS · Anvisa RDC 657 | Requisitos brasileiros (docs [08](08-seguranca-lgpd-etica.md) e [14](14-regras-de-arquitetura-e-engenharia.md)) |

---

## Checklist de proteção do usuário (no PR, além do G1–G6)
- [ ] Risco clínico avaliado e registrado (ou "não se aplica" justificado)
- [ ] Requisito com ID e teste que o comprova
- [ ] Dois identificadores em toda tela/documento clínico tocado
- [ ] Erro humano prevenido onde possível (R1); confirmação só em ato irreversível e com o nome do paciente (R3)
- [ ] Nível de alerta correto (R4); nenhum bloqueante novo sem aprovação clínica
- [ ] Rascunho e reenvio sem perda (R6); atendimento segue sem dependência externa (R7)
- [ ] Mensagens de erro no formato R8; a tela nunca mente
- [ ] Acessibilidade (R10) e ética com o paciente (R11) verificadas
- [ ] Mudança visível comunicada e treinamento previsto (R9, R12)
- [ ] Lançamento com prontidão, piloto e chave liga/desliga (C5)
