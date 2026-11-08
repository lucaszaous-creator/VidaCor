# Qualidade de código, PR, plano de correção e bugs

Fontes: `docs/14-regras-de-arquitetura-e-engenharia.md` B1 a B7; `docs/15-padrao-de-qualidade.md` (G5, G6 e checklist);
`docs/16-protecao-do-usuario.md` (checklist); `docs/17-seguranca-por-classe-de-ataque.md` Parte C; `docs/playbooks/`.

## 1. Fluxo de trabalho

- **Trunk-based** com branches curtas (até 2 dias); `main` sempre implantável e protegida.
- **PR pequeno**: alvo de até 400 linhas alteradas, com descrição, evidência (teste e print) e vínculo à tarefa.
- **Conventional Commits** e **SemVer**; changelog gerado.
- Revisão por pelo menos 1 pessoa; **2 pessoas** em laudos, assinatura, faturamento, caixa, auditoria, identidade, exames
  e DICOM (`CODEOWNERS`).
- **RFC** antes de funcionalidade grande; **ADR** para decisão estrutural.
- Executor único: `just dev`, `just test`, `just lint`.

## 2. Limites e estrutura

- Arquivo acima de **400 linhas** pede revisão de responsabilidade; acima de **600**, o PR justifica ou divide (G5).
- Funções curtas com uma responsabilidade; nomes no idioma do domínio; comentário explica o porquê; sem código morto ou comentado.
- Cobertura mínima: **80%** nos crates de domínio, **90%** em laudos, faturamento, caixa e assinatura (CI).

## 3. Uso de assistentes de IA (docs/14 B6)

- Mesmas regras de qualquer código: revisão humana, testes, lint e cobertura. Quem abre o PR responde pelo código gerado.
- Assistentes seguem o `CLAUDE.md` e os playbooks: auditoria só lê; correção só depois de plano aprovado (G6).
- Proibido colar dado de paciente, segredo ou código sensível em ferramenta externa sem contrato de privacidade.

## 4. Refatoração segura (G5)

1. Testes de **caracterização** do comportamento atual antes de mexer.
2. Um passo por vez, cada passo com testes verdes.
3. Mesmos nomes públicos, rotas e colunas; refatoração não muda comportamento.
4. Bug encontrado vai para `docs/bugs.md`, não é corrigido no mesmo PR.
Roteiro completo: `/refatoracao-segura` (`docs/playbooks/refatoracao-segura.md`).

## 5. Modelo de plano de correção (G6)

Entregue ao dono antes de alterar código, migration ou proteção. Só comece com o OK registrado.

```markdown
# Plano: <título curto>

- Origem: <achado da auditoria, bug BUG-NNN, frase FC-*, requisito REQ-*>
- Garantias e regras tocadas: <G1..G6, SEC-*, R*, docs/21>
- Risco clínico: <ID do registro ou "não se aplica" com motivo>

## Situação atual
O que acontece hoje, com arquivo e linha. O que não foi possível verificar: "não verificado" e o motivo.

## Mudança proposta, em passos
1. Testes de caracterização ou testes das frases aprovadas que vão provar a correção.
2. Passo de código 1 (arquivos).
3. Migration, se houver (texto da migration para revisão humana; nenhum dado apagado).

## Como verificar
Comandos (`just test`, teste específico) e resultado esperado.

## Como desfazer
Reverter o commit; migration expand/contract; chave liga/desliga.

## Fora do escopo
Bugs vistos e registrados em docs/bugs.md, não corrigidos aqui.

Aprovação do dono: [ ] data:
```

## 6. Modelo de descrição de PR

```markdown
## O que muda
Resumo em 3 a 5 linhas. Requisitos: REQ-... Frases: FC-... Riscos: RC-...

## Evidências
Testes adicionados (nomes), resultado de `just test`, prints das telas em 1440x900 e 1280x800.

## Seis garantias (docs/15)
- [ ] G1 regra no servidor e recalculada; nenhum segredo na tela
- [ ] G2 recurso limitado protegido no banco, com teste de duas requisições simultâneas
- [ ] G3 webhook com corpo bruto e ID único antes de agir; chave de idempotência; Idempotency-Key nas rotas de escrita
- [ ] G4 integração na rota real: feliz, erro, repetição, limite, acesso indevido; nada pulado ou afrouxado
- [ ] G5 tela sem regra, domínio sem I/O, arquivos dentro do limite
- [ ] G6 migration revisada; nada apagado; plano aprovado (link)

## Proteção do usuário (docs/16)
- [ ] Risco clínico avaliado; requisito com ID e teste
- [ ] Dois identificadores; R1 prevenção; R3 confirmação com nome do paciente; R4 nível de alerta
- [ ] R6 rascunho e reenvio; R7 atendimento segue; R8 mensagens; R10 acessibilidade; R11 ética
- [ ] Mudança visível comunicada; treinamento previsto; chave liga/desliga

## Segurança (docs/17 Parte C)
Domínios tocados: B1 [ ] B2 [ ] B3 [ ] B4 [ ] B5 [ ] B6 [ ] B7 [ ] B8 [ ] B9 [ ] B10 [ ] B11 [ ] B12 [ ] B13 [ ]
IDs atendidos: SEC-...

## Lastro (docs/21)
- [ ] Metadados, eventos e ligações gravados e testados; repetir não duplica

## Documentação
- [ ] OpenAPI  [ ] ADR/RFC  [ ] manual  [ ] matriz de rastreabilidade  [ ] docs/decisoes.md

## Como desfazer
```

## 7. Registro de bug em `docs/bugs.md`

Teste que falha por comportamento errado do sistema **continua falhando** (G4). Acrescente uma linha na tabela, sem apagar
as anteriores:

```markdown
| BUG-0001 | Quando duas recepcionistas agendam o mesmo horário da sala 1, os dois agendamentos são criados | `agenda::dois_agendamentos_simultaneos_mesmo_horario` | 2026-11-20, PR <link> | Aberto |
```

Campos: ID sequencial, descrição no formato "quando..., então..." do comportamento observado, teste que reproduz, onde e
quando foi encontrado, status (Aberto, Em correção, Corrigido em <PR>). Bug com efeito clínico vai também ao responsável
clínico de segurança no mesmo dia (docs/16 C7).

## 8. Definition of Done (docs/14 B2)

1. Código revisado e mesclado; testes passando na CI.
2. Critérios de aceite verificados em homologação.
3. Acessibilidade verificada (teclado, leitor de tela, contraste, axe).
4. Métricas, logs e traces instrumentados; orçamento de latência respeitado.
5. Documentação atualizada (OpenAPI, ADR ou RFC, manual se mudou tela).
6. Migration reversível ou plano de contorno documentado.
7. Feature flag definida quando aplicável.
8. Checklist G1 a G6 marcado no PR.
