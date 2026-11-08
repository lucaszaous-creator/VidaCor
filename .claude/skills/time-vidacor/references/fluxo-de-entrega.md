# Fluxo de entrega: os portões em detalhe

Cada portão tem entrada, verificação, bloqueios e saída. "Bloqueia" significa que o item não passa,
mesmo com prazo apertado. Exceção só escrita no PR e aprovada por quem assina o portão.

## P0 Ideia registrada

- **Entrada:** pedido de alguém da clínica, achado de usabilidade (`ACH-USA-*`), glosa recorrente, incidente, item do roteiro (`docs/22-lacunas-e-roadmap.md`).
- **Verificar:** quem sofre o problema, em que momento do dia, como é feito hoje (Medware, Medilab, manual), qual módulo do `docs/11` § 5 é afetado, se está no MVP (`docs/25` § 2).
- **Bloqueia:** pedido que não diz qual problema real resolve.
- **Saída:** item no backlog com módulo, fase e prioridade proposta. Fora do MVP vai para "depois", salvo decisão do comitê (`docs/25` § 9.3).

## P1 Design aprovado

- **Entrada:** item priorizado.
- **Verificar:**
  - documento de design pelo modelo `docs/modelos/documento-de-design.md`, começando pelo comunicado "de trás para frente";
  - risco clínico no registro (`docs/modelos/registro-de-riscos-clinicos.md`); risco 12 ou mais sem controle não é construído;
  - STRIDE quando toca dado pessoal, dinheiro, integração ou permissão (`docs/16` C2, `docs/14` A9);
  - tela no mockup `.dc.html`, com estados, menu de contexto e dois identificadores quando clínica;
  - níveis de dado e perfis afetados (`docs/18`), lastro previsto (`docs/21`).
- **Bloqueia:** risco alto sem controle; tela clínica sem dois identificadores; mudança de fluxo clínico sem o responsável clínico.
- **Saída:** design com status "aprovado" e as aprovações marcadas no próprio documento.

## P2 Pronto para começar (Definition of Ready, `docs/14` B2)

- **Verificar:**
  - requisitos com ID (`REQ-<MOD>-NNN`) e critério de aceite verificável;
  - frases de comportamento (`FC-*`) cobrindo caminho feliz, erro, repetição, limite e acesso indevido, aprovadas pelo dono do produto (e pelo responsável clínico quando clínicas);
  - tarefas críticas de usabilidade identificadas (`docs/26` § 9);
  - impacto em LGPD avaliado;
  - dependências externas conhecidas (operadora, PSP, Meta, prefeitura, provedor de assinatura).
- **Bloqueia:** frase "aguardando aprovação"; requisito sem critério; regra de negócio não validada com a clínica.

## P3 Plano técnico

- **Verificar:** módulos e crates afetados; tabelas e migrations (aditivas, expand/contract); eventos de domínio e versão; integrações e idempotência; garantias G1 a G6 aplicáveis, uma a uma; domínios SEC-* tocados (`docs/17` Parte C); RLS e testes de RLS; métricas e alertas novos; chave liga/desliga.
- **Bloqueia:** proteção de exclusividade fora do banco (G2); regra de preço, desconto, permissão ou limite na tela (G1); componente novo sem medição e sem ADR (`docs/14` A1).
- **Saída:** plano no PR ou no documento de design; ADR quando estrutural.

## P4 Construído

- **Verificar:** testes escritos a partir das frases antes ou junto do código; PR pequeno (alvo 400 linhas); revisão por uma pessoa e por duas nos módulos críticos (laudos, assinatura, faturamento, caixa, auditoria, identidade, exames e DICOM); migration revisada por pessoa antes de rodar; checklists do PR (`docs/15`, `docs/16`, `docs/17` Parte C, `docs/28` § 4).
- **Bloqueia:** teste pulado ou afrouxado; `unwrap` em produção; segredo no código; arquivo acima de 600 linhas sem justificativa.

## P5 Verificado

- **Verificar:** `just test` verde ou falha de bug real registrada em `docs/bugs.md`; cobertura (80% nos domínios, 90% em laudos, faturamento, caixa e assinatura); acessibilidade (axe e teclado); p95 dentro do orçamento (`docs/12` § 5); matriz de rastreabilidade atualizada; testes de lastro (eventos e ligações gravados, repetir não duplica).
- **Bloqueia:** requisito "implementado" sem teste; teste instável não tratado.

## P6 Aceito

- **Verificar:** critérios comuns do `docs/25` § 3.2 e específicos do § 3.3; teste de usabilidade com 4 de 5 sem erro e sem ajuda e nenhum erro crítico; demonstração em homologação na revisão quinzenal.
- **Bloqueia:** erro crítico em tarefa crítica; parecer clínico ausente em módulo clínico.
- **Saída:** aceite assinado pelo dono do produto e pelo usuário-chave do setor.

## P7 Pronto para produção

- **Verificar:** todos os itens de `docs/modelos/prontidao-para-producao.md`: treinamento, manual, painéis e alertas testados, manual de operação, capacidade, backup antes e volta testada, chave e canal piloto, janela e dono técnico disponível, varreduras limpas.
- **Bloqueia:** qualquer item sem marcar.

## P8 Lançado

- **Verificar:** canal piloto (uma ou duas máquinas) com comparação automática de erros e latência; liberação gradual por perfil, máquina ou sala; fora de véspera de feriado, fechamento do faturamento ou incidente aberto (`docs/16` C5).
- **Bloqueia:** congelamento ativo; piora no canário (volta sozinha).

## P9 Operando

- **Verificar:** SLO e orçamento de erro (`docs/12`); orçamento estourado significa só correções até normalizar (`docs/16` C6); exercícios de restauração, troca de servidor e incidente; pós-incidente para S1, S2 e todo incidente clínico (`docs/16` C7).
- **Saída:** toda ação de pós-incidente vira teste, regra no `CLAUDE.md`, verificação na CI ou risco no registro.

## Itens que não seguem o ciclo inteiro

| Tipo | Portões obrigatórios |
|---|---|
| Correção de bug sem mudança de comportamento | P3 (plano aprovado, G6), P4, P5, P7 e P8 |
| Refatoração | P3 com testes de caracterização antes (G5), P4, P5; nenhuma mudança de comportamento |
| Mudança só de documentação | Revisão no PR; se mudar regra, linha em `docs/decisoes.md` |
| Correção urgente de segurança | P3 resumido, P4, P5; P7 e P8 na janela de emergência com o dono técnico; pós-incidente depois |
| Auditoria | Somente leitura; relatório pelo `docs/playbooks/modelo-auditoria.md`; correções viram itens novos |
