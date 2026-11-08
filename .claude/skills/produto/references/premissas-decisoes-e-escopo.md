# Premissas, decisões e mudança de escopo

Fontes: `docs/25-plano-do-projeto.md` §1 e §9.3; `docs/decisoes.md`;
`docs/adr/0019-decisoes-pelo-mais-profissional-com-lastro.md`; `docs/adr/README.md`.

## 1. Premissas (PREM-*)

A tabela do `docs/25` §1 tem 26 premissas (PREM-01 a PREM-26), todas "a confirmar" pelo
dono do produto. Elas sustentam equipe, orçamento, prazo, escopo e contrato.

Grupos para conversar com o dono:

| Grupo | Premissas | Afeta |
|---|---|---|
| Contrato e equipe | PREM-01, 03, 04, 20, 21, 22, 25, 26 | Orçamento, propriedade do código, pagamento |
| Operação da clínica | PREM-02, 19, 23 | Carga, disponibilidade clínica, SLA |
| Prazo | PREM-05, 08, 09 | Cronograma, licenças antigas |
| Infraestrutura | PREM-06, 16 | Compra de hardware, nuvem e backup |
| Integrações e custos recorrentes | PREM-07, 10, 11, 12, 13, 14, 15 | Escopo da Fase 2, custos mensais |
| Escopo do MVP | PREM-17, 18, 24 | TISS no MVP, visualizador, apps e telemedicina |

Regras:
- Confirmar premissa: o dono escreve "confirmada" (ou o valor real); o PO muda a situação na tabela, registra linha em `docs/decisoes.md` e **recalcula o que depende dela** (orçamento, cronograma, escopo).
- Premissa que mudou não é sobrescrita: a situação vira "Substituída" com referência à decisão nova.
- Nunca apresentar número do `docs/25` §6 como cotação: são faixas de referência "não verificadas".

## 2. Registro de decisões

Regra do ADR-0019: toda escolha entre caminhos é registrada em `docs/decisoes.md` com data,
quem decidiu, alternativas, motivo, onde foi aplicado e como desfazer. Nada é apagado.

Formato da linha (colunas da tabela existente):

```markdown
| DEC-AAAA-MM-DD-NN | DD/MM/AAAA | <decisão em uma frase> | <alternativas consideradas> | <motivo> | <quem decidiu: "Dono do produto (Lucas)" ou "Claude (pela regra ADR-0019)"; "a confirmar" quando depende de premissa> | <onde foi aplicado: docs, telas, arquivos> | <como desfazer> | <DEC substituída ou —> |
```

Numeração: sequencial por dia; conferir a última linha antes de criar. Decisão que muda outra
cria linha nova e preenche "Substitui"; a antiga continua na tabela.

Quem pode decidir sozinho (assistente, pela ADR-0019): forma, organização, redação, escolha
entre alternativas equivalentes já cobertas por regra. Quem decide o resto: dono do produto
(comportamento, prioridade, exceção), responsável clínico (fluxo clínico), tech lead e dono
(ADR), comitê (escopo, prazo, orçamento).

## 3. Quando vira ADR

Decisão estrutural (arquitetura, tecnologia, padrão transversal, nova infraestrutura,
mudança de garantia) vira ADR em `docs/adr/NNNN-<titulo>.md`, com status "proposto" até a
assinatura do tech lead e do dono do produto, e entra no índice `docs/adr/README.md`.
O PO não redige o design técnico do ADR; ele garante que a decisão de produto que motivou o
ADR esteja registrada e que o ADR seja assinado.

Exemplos: infraestrutura nova para exclusividade (G2 só permite com ADR); integração fora do
perfil IHE (`docs/16` C3.2); cópia local da agenda nas estações (DEC-2026-11-08-14 exige ADR).

## 4. Mudança de escopo (`docs/25` §9.3)

Nada entra no MVP por conversa de corredor. Todo pedido segue:

1. Pedido escrito com motivo.
2. Análise de impacto em prazo, custo e risco (técnico com `desenvolvimento` e `devops-seguranca`; clínico com o responsável clínico, se for o caso).
3. Decisão do comitê do projeto, com aprovação da direção quando muda prazo ou orçamento.
4. Linha em `docs/decisoes.md` e atualização do `docs/25` (§2, §3, §5 ou §6, conforme o caso).
5. Itens recusados ficam registrados com o motivo; itens adiados vão para "Depois do MVP".

Modelo do pedido:

```markdown
## Pedido de mudança de escopo PMS-AAAA-NNN
- Data do pedido: DD/MM/AAAA | Quem pediu: <nome e papel>
- O que se pede: <descrição>
- Motivo: <problema real da clínica>
- Classe: inclusão / retirada / troca / mudança de comportamento aprovado
- Itens afetados: <épicos, REQ-*, FC-*, fases, marcos>
- Impacto em prazo: <semanas, com premissas>
- Impacto em custo: <faixa, com premissas>
- Impacto em risco: <clínico (RC-*), segurança, projeto (docs/25 §8)>
- Alternativas: <adiar para depois do MVP, reduzir, trocar por outro item>
- Recomendação do PO: <proposta>
- Decisão do comitê: <aprovado/recusado/adiado>, em <data>, por <nomes>
- Registro: DEC-AAAA-MM-DD-NN
```

Mudança em comportamento já aceito, depois do aceite do módulo, é mudança de escopo e não
defeito (`docs/25` §7.6).

## 5. Exceção a regra

Quando uma regra do `docs/16` não puder ser cumprida: exceção escrita no PR, aprovada pelo
dono do produto e, se clínica, pelo responsável clínico, e registrada em `docs/decisoes.md`.
Exceção às seis garantias G1 a G6 não é aceita por esta via: G2 só com ADR, e as regras
marcadas como "nunca" ou "proibido" no `CLAUDE.md` não têm exceção.

## 6. Perguntas que só o dono responde

Leve sempre proposta pronta, com premissas marcadas "a confirmar":
- valores, prazos e contratos reais (PREM-01 a PREM-26);
- se a clínica faz consulta hoje (atendimento médico no MVP);
- lista final de perfis, prazo do vínculo de cuidado, limites de desconto (`docs/18` §9);
- unidades (uma ou várias);
- aprovação de frases, de aceite e de exceções.
