# Requisitos, histórias e documento de design

Fontes: `docs/16-protecao-do-usuario.md` C2; `docs/14-regras-de-arquitetura-e-engenharia.md` B2;
`docs/modelos/documento-de-design.md`; `docs/modelos/rastreabilidade.md`;
`docs/modelos/registro-de-riscos-clinicos.md`.

## 1. Documento de design de trás para frente

Todo módulo ou função grande começa pelo documento de design (`docs/16` C2.1). O PO escreve
as seções 1 a 3, 7 e 8; `ux-ui` a seção 4; `desenvolvimento` a seção 6; riscos (seção 5) são
reunidos por todos e aprovados pelo responsável clínico quando clínicos.

Ordem de escrita:
1. **Comunicado:** um parágrafo, como se já estivesse pronto, dizendo o que muda para quem usa.
2. **Perguntas do usuário:** quem usa, em que momento do dia, como é feito hoje, como saberá que deu certo.
3. **Requisitos com ID** e critério de aceite.
4. Telas e tarefas críticas (com `ux-ui`).
5. Riscos clínicos, de segurança (STRIDE) e de operação.
6. Design técnico (com `desenvolvimento`).
7. Lançamento: chave liga/desliga, piloto, treinamento, plano de volta.
8. Fora do escopo.

Aprovações no topo do documento: dono do produto, responsável clínico (se clínico), dono técnico.

## 2. Identificador de requisito

Formato: `REQ-<MOD>-NNN`, número com três dígitos, nunca reaproveitado.

Prefixos já em uso nos docs: `PAC` (pacientes), `EXA` (exames), `LAU` (laudos) na matriz de
rastreabilidade; `AGD`, `CHK`, `PUL`, `TRI`, `CXA`, `CTG` nas frases do `docs/27`; `USA`
(usabilidade), `CONT` (contingência) e `MIG` (migração) nos documentos de processo.

Prefixos propostos para os demais módulos do `docs/11` §5 (a confirmar, registrar quando
usados pela primeira vez): `IDT` identidade, `AUD` auditoria, `REC` recepção, `ETQ`
etiquetas, `ATD` atendimento, `FAT` faturamento, `FIN` financeiro, `EST` estoque, `COM`
comunicação, `IND` indicadores, `POR` portal, `CFG` configurações, `AUT` autorização (E4),
`EXT` exame externo (E3), `ESC` estacionamento.

Regras:
- Requisito retirado fica com status "Retirado" e motivo; o ID não volta a ser usado.
- Requisito mudado depois de aprovado ganha nova linha com sufixo de versão (`REQ-AGD-012 v2`).
- Requisito com efeito clínico aponta para o risco que controla (`RC-*` ou `RCP-*`).

## 3. Como escrever um requisito bom

| Característica | Teste rápido |
|---|---|
| Um comportamento só | Tem "e" ligando duas regras? Divida |
| Verificável | Dá para escrever o teste sem perguntar nada a ninguém? |
| Sem solução técnica | Fala do que a clínica precisa, não de tabela ou componente |
| Diz quem decide | Preço, desconto, permissão e limite: "o servidor calcula/recusa" (G1) |
| Diz quem pode | Perfil e nível de dado (`docs/18`) |
| Diz o que fica no lastro | Eventos, ligações, motivo (`docs/21`) |
| Diz o que o usuário vê na falha | Mensagem R8: o que aconteceu, o que foi feito, o que fazer |
| Cobre as cinco classes | Feliz, erro, repetição, limite, acesso indevido (G4) |

Palavras proibidas sem número: "rápido", "fácil", "intuitivo", "seguro", "robusto". Troque
por métrica: "p95 até 150 ms" (`docs/12` §5), "4 de 5 sem ajuda" (`docs/26` §11).

## 4. História de usuário

A história é o formato de conversa; o requisito com ID é o formato de contrato. Toda história
gera pelo menos um requisito e uma frase.

```markdown
### História: <título curto>
Como <perfil do docs/18>, no momento <quando no dia>,
quero <ação>,
para <resultado para o paciente ou para a clínica>.

- Épico: <módulo> | Fase/marco: <...>
- Hoje é feito: <Medware/Medilab/papel/não existe>
- Requisitos: REQ-<MOD>-NNN, ...
- Frases: FC-<MOD>-NN, ...
- Tarefa crítica de usabilidade: <T-XXX-NN ou "não">
- Risco clínico: <RC-xxx ou "não se aplica" + motivo>
- Lastro: <eventos e ligações esperados>
- Fora do escopo: <...>
```

## 5. Modelo de requisito

```markdown
| ID | Requisito | Critério de aceite | Perfil | Risco | Frases | Situação |
|---|---|---|---|---|---|---|
| REQ-AGD-012 | O agendamento de convênio só é confirmado com guia autorizada e com saldo para cada procedimento; o servidor consome o saldo de forma atômica | Duas requisições simultâneas com a mesma guia: só uma cria agendamento, a outra recebe 409; saldo nunca negativo | Recepção | — | FC-AGD-01, FC-AGD-03, FC-AGD-07 | Proposto |
```

Situações possíveis: Proposto, Aprovado (com nome e data), Implementado, Verificado,
Retirado. Só o dono do produto muda para Aprovado.

## 6. Exemplo com dados fictícios

```markdown
### História: check-in com dois identificadores
Como recepcionista (Ana Souza), na chegada do paciente,
quero confirmar nome completo e nascimento antes de concluir o check-in,
para nunca abrir atendimento no paciente errado.

- Épico: Recepção | Fase 1 | M1
- Hoje é feito: Medware, sem bloqueio
- Requisitos: REQ-CHK-001 (check-in bloqueado sem os dois identificadores), REQ-CHK-002 (alerta de homônimo no mesmo dia)
- Frases: FC-CHK-01 em diante (`docs/27` §2)
- Tarefa crítica: T-REC-03, T-REC-04
- Risco clínico: RC-001
- Lastro: evento de check-in com quem, estação, horário e confirmação dos identificadores
```

Exemplo de homônimo usado nos mockups: "Maria Helena Duarte", nascida em 12/03/1968, e o
duplicado proposital com nascimento 21/03/1968.

## 7. Matriz de rastreabilidade

Arquivo: `docs/modelos/rastreabilidade.md` (IEC 62304, versão enxuta). Colunas: requisito,
descrição, risco relacionado, módulo/crate, testes, status. A CI falha se requisito
"implementado" não tiver teste. O PO:
- cria a linha quando o requisito é aprovado;
- confere, no aceite, que todo requisito do módulo tem teste verde;
- liga os achados de usabilidade (`ACH-USA-*`) ao requisito que criaram ou mudaram.

## 8. Definition of Ready (resumo do `docs/14` B2)

Problema e critério de aceite claros; tela no mockup ou esboço; regras de negócio validadas
com a clínica; impacto em LGPD avaliado. O checklist completo do papel está no `SKILL.md` § 7.

## 9. Requisitos não funcionais que todo módulo herda

Não reescreva em cada história; cite o critério comum do `docs/25` §3.2:
- latência p95 abaixo de 150 ms nas telas operacionais (`docs/11` R2);
- WCAG 2.2 AA verificado com axe e teclado;
- tela cabe em 1440x900 e 1280x800 sem rolar a página (`docs/05`);
- sem rolagem lateral em 1280 px ou mais;
- menu de contexto em todo objeto, com ações vindas do servidor (`docs/19`);
- dado de saúde nunca em log, métrica, URL ou cache do navegador.

Só escreva requisito não funcional próprio quando o módulo for mais exigente (por exemplo,
abertura de estudo no visualizador em p95 até 2 s, `docs/12` §5).
