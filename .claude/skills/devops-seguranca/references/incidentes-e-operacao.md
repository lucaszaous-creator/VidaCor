# Incidentes e operação

Fontes: `docs/16-protecao-do-usuario.md` C6, C7, R13 e R14, `docs/14-regras-de-arquitetura-e-engenharia.md` B7 e B8,
`docs/25-plano-do-projeto.md` §7.1 e §7.2, `docs/modelos/pos-incidente.md`.

## 1. Gravidade e tempos

| Gravidade | Exemplo | Primeira resposta | Solução |
|---|---|---|---|
| S1 | Atendimento parado, risco ao paciente | 15 min, 24 h por dia | Contorno em até 4 h; pós-incidente em até 5 dias úteis |
| S2 | Módulo indisponível | 1 h no horário de funcionamento | Pós-incidente obrigatório |
| S3 | Lentidão com contorno | 1 dia útil | |
| S4 | Incômodo, sugestão | 5 dias úteis | |

Todo incidente clínico (identificação errada, resultado errado ou no lugar errado) tem pós-incidente,
qualquer que seja a gravidade técnica, e é comunicado ao responsável clínico de segurança **no mesmo dia**
(docs/16 C7). Incidente com dado pessoal é comunicado à clínica em até 24 h da ciência (docs/25 §7.5).

## 2. Resposta

1. **Detectar e classificar**: alerta ou chamado; definir gravidade; abrir o registro do incidente com
   número, responsável e hora de início.
2. **Coordenar**: um responsável pelo incidente; quem investiga não é quem comunica, quando houver gente
   suficiente.
3. **Proteger o atendimento primeiro**: contorno antes de causa raiz. Se a volta estimada passar de 15
   min, informar a coordenação para decidir a contingência (docs/23 §3).
4. **Comunicar** no formato R13 (§ 3), pelo painel e pelo canal de suporte, e atualizar a cada mudança
   relevante.
5. **Não apagar evidência**: preservar logs, traces e estado; nenhuma ação destrutiva para "limpar".
6. **Não desligar proteção** para resolver mais rápido (G6). Se uma proteção estiver causando o
   incidente, a exceção é decidida pelo dono técnico e pelo dono do produto e registrada.
7. **Encerrar** com hora de resolução e retorno a quem reportou (docs/16 R14).

## 3. Comunicado de incidente (template R13)

```markdown
Assunto: [S<gravidade>] <módulo>: <o que está acontecendo, em uma frase>

O que aconteceu: <descrição em português da clínica, sem código técnico>
Impacto: <setores, telas e atos afetados; se há risco ao paciente>
O que fazer enquanto isso: <contorno: outra estação, kit de contingência, aguardar; quem procurar>
Previsão de volta: <hora estimada ou "nova atualização às HH:MM">
Código de suporte: <número do incidente>
Próxima atualização: <HH:MM>
```

Depois da resolução, enviar um resumo da causa e do que foi feito para não repetir.

## 4. Pós-incidente sem culpados

Modelo em `docs/modelos/pos-incidente.md`: gravidade, se é clínico, linha do tempo, fatos, 5 porquês, o
que funcionou e o que não funcionou, ações e comunicação feita.

Regra das ações (docs/16 C7): toda ação vira **teste, regra no `CLAUDE.md`, verificação na CI, risco no
registro de riscos clínicos ou processo**, com dono e prazo. Ação "tomar mais cuidado" não é aceita.

## 5. Runbook de serviço (template)

Cada serviço tem runbook em `infra/runbooks/<servico>.md` (docs/14 B7). O link aparece na anotação do
alerta.

```markdown
# Runbook: <serviço>
Dono técnico: <nome>  Última revisão: <data>  Versão do serviço: <versão>

## O que é
<função, de quem depende, quem depende dele>

## Sinais de saúde
- Health: <endpoint ou comando>
- Painel: <nome do painel no Grafana>
- SLO: <meta>

## Alertas e o que fazer
| Alerta | Significado | Primeiro passo | Escalar para |
|---|---|---|---|

## Procedimentos
### Reiniciar
### Diagnosticar
<logs e traces por trace_id; consultas permitidas; nunca abrir dado clínico>
### Restaurar
<referência ao procedimento de backup e volta>
### Trocar para a contingência

## Dependências externas e contatos
## Histórico de incidentes
<números e links dos pós-incidentes>
```

## 6. Operação contínua

- **SLO e orçamento de erro** revisados mensalmente; orçamento estourado congela funções novas até
  normalizar (docs/16 C6).
- **DORA** acompanhado mensalmente: frequência de implantação, tempo de entrega, taxa de falha de mudança,
  tempo de restauração (docs/14 B8, docs/12 §3.7).
- **Redução de trabalho manual**: tarefa operacional feita à mão mais de 3 vezes vira automação ou
  runbook (docs/16 C6).
- **Exercícios**: restauração mensal, troca para a contingência trimestral, incidente de segurança
  semestral, simulado de contingência trimestral (docs/23 §10).
- **Capacidade**: projeção de esgotamento de disco do PACS e do WAL revisada mensalmente.
- **Relatório mensal ao comitê** inclui disponibilidade, incidentes, SLO e ações em aberto (docs/25 §9.4).

## 7. Suporte

- Canal único de chamados com número, gravidade, responsável e retorno do desfecho a quem abriu.
- Plantão para S1 fora do horário.
- Todo erro mostrado ao usuário carrega o `trace_id`; a TI localiza o caso sem pedir dado do paciente.
- Atualizações só em janela de manutenção, com volta em um comando (docs/25 §7.2).

## 8. Checklist de encerramento de incidente

- [ ] Hora de início, detecção e resolução registradas
- [ ] Comunicação R13 inicial e de encerramento enviadas
- [ ] Responsável clínico informado no mesmo dia, se clínico
- [ ] Pós-incidente agendado (S1, S2 e todo clínico)
- [ ] Ações com tipo, dono e prazo
- [ ] Quem reportou recebeu o desfecho
