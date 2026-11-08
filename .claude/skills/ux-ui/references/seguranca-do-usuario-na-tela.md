# Segurança do usuário na tela (R1 a R14 como decisões de desenho)

Fonte: `docs/16-protecao-do-usuario.md`, Parte 2. Cada item é **requisito de interface**, verificado
na revisão de código e no teste de usabilidade. Exemplos com pacientes de `design/dados-ficticios.md`.

## R1. Prevenir em vez de avisar

| Situação | Decisão de desenho |
|---|---|
| Horário ocupado | Não aparece como livre; não há botão "agendar" nele |
| Estudo sem paciente conciliado | O laudo não abre; a fila mostra "Aguardando conciliação" com quem resolve |
| Alergia registrada na prescrição | Ato bloqueado até a justificativa do médico |
| Caixa fechado | Botão de receber indisponível, com o motivo e o atalho "Abrir caixa" |
| Triagem "precisa de aprovação médica" | Exame bloqueado na sala até o médico registrar a conduta (E1) |
| Campo de CPF, data, placa, valor | Máscara e formato explicados antes; valor inválido não é aceito |

## R2. Dois identificadores sempre visíveis

Nome completo e nascimento (e foto, quando houver) no topo de toda tela clínica e em todo impresso.
Homônimos do dia recebem alerta ("Maria Helena Duarte, 12/03/1968" e "Maria Helena Duarte,
21/03/1968" no mesmo dia: mostrar os dois nascimentos lado a lado).

## R3. Confirmação só onde importa

Atos com confirmação: assinar e liberar laudo, estornar, cancelar exame, publicar resultado, fechar
caixa, vincular imagem manualmente. A confirmação diz o quê e com quem; o botão repete o verbo.

```
Título:  Assinar e liberar o laudo
Texto:   Assinar o laudo de Sônia Maria Castro, 03/02/1962, ecocardiograma de 07/11/2026,
         e liberar no portal?
         A assinatura é definitiva. Correções depois geram nova versão com motivo.
Botões:  [Assinar e liberar]   [Voltar ao laudo]
```

Proibido: "Tem certeza?", "OK/Cancelar", confirmação em ação comum.

## R4. Três níveis de alerta

| Nível | Quando | Desenho |
|---|---|---|
| Bloqueante | Risco ao paciente ou perda de dinheiro | Impede o ato até resolver ou justificar; texto do que fazer |
| Atenção | Precisa de ação hoje | Etiqueta na tela, sem interromper |
| Informativo | Contexto útil | Texto discreto |

Bloqueante novo exige aprovação do responsável clínico de segurança. Cor e texto juntos; laranja e
azul quando a diferença importa (docs/05).

## R5. Desfazer e corrigir, nunca apagar

- Ação comum (mover agendamento, chamar senha errada): aviso curto com "Desfazer" por alguns segundos.
- Registro clínico e financeiro: não há apagar nem desfazer; há "Retificar" ou "Estornar", com
  motivo, gerando nova versão. O histórico fica visível para quem tem permissão.

## R6. Nada se perde

- Rascunho automático em laudo, evolução, cadastro e recebimento, com a hora do último salvamento.
- Queda de rede: aviso fixo, o que foi digitado fica, reenvio ao reconectar sem duplicar (G3).
- Trocar de paciente pela barra salva o rascunho antes.

## R7. O atendimento nunca para

Sem internet, convênio, PIX, WhatsApp ou NFS-e, a tela deixa seguir e coloca o que falta numa
**lista de pendências** visível, com quem resolve e quando. Falha de um módulo não derruba a tela
dos outros. Contingência total segue `docs/23-modo-contingencia.md` (mockup `Contingencia`).

## R8. Mensagens de erro que ajudam

Formato obrigatório: **o que aconteceu, o que já foi feito, o que fazer agora**, em português da
clínica, sem código técnico na frente; o código de suporte (`trace_id`) aparece discreto ao final.

Exemplo do docs/16: "O convênio não respondeu à consulta de elegibilidade. O check-in foi concluído
e a consulta será refeita automaticamente. Se precisar, ligue para a central do convênio."

A tela nunca mente: falha não aparece como sucesso; ausência de dado não aparece como zero;
"Enviado" só depois da resposta do servidor.

### Modelo: catálogo de mensagens da tela

```
Catálogo de mensagens: <Tela> (<mockup .dc.html>), versão <data>

| Código estável | Situação | O que aconteceu | O que já foi feito | O que fazer agora | Nível | Frase FC ligada |
|---|---|---|---|---|---|---|
| AGD-GUIA-EM-USO | Guia já consumida | A guia UN-55120 já está em uso no atendimento ATD-2026-118790. | Nada foi agendado. | Peça outra guia ao paciente ou reserve o horário provisoriamente. | Bloqueante | FC-AGD-02 |
| AGD-SENHA-VENCIDA | Senha vencida na data | A senha da guia vale até 20/11/2026 e o exame está marcado para 25/11/2026. | Nada foi agendado. | Remarque até 20/11/2026 ou peça prorrogação ao convênio. | Bloqueante | FC-AGD-06 |

Regras: código estável igual ao do Problem Details da API (docs/14 A4); texto sem jargão técnico;
nenhum dado de saúde na mensagem que vá para log ou notificação.
```

## R9. Consistência e previsibilidade

Mesma ação, mesmo lugar, mesmo nome em todas as telas. Mudança visível é avisada antes à equipe.

### Modelo: nota de mudança visível para a equipe

```
Mudança na tela <Tela>, a partir de <data> (versão <x.y>)

O que muda: <uma ou duas frases, no vocabulário da clínica>.
Por quê: <motivo, por exemplo achado ACH-USA-2026-004 ou pedido do setor>.
O que você faz diferente: <passo a passo curto>.
Quem é afetado: <perfis e setores>.
Treinamento: <vídeo curto ou sessão, data>.
Dúvidas: <canal de suporte>.
```

## R10. Acessibilidade e conforto

WCAG 2.2 AA, teclado completo, foco visível, contraste, nada espremido, letra ajustável no portal e
no app. Detalhe em `references/acessibilidade.md`.

## R11. Ética com o paciente

- Resultado delicado: o médico marca "conversar antes" e o laudo não vai ao portal nem ao app até a
  liberação manual (RC-004). A tela mostra claramente que está retido e por quê.
- IA: rascunho de IA aparece **marcado como rascunho de IA**, em área distinta, e só vira laudo
  depois de editado ou aprovado pelo médico (SEC-IA-2). Sem ícone de brilho, varinha ou estrela.
- Portal: o paciente vê quem acessou seus dados, baixa os próprios dados e controla consentimentos.
- Nenhum padrão enganoso: nada pré-marcado para marketing; cancelar consentimento tão fácil quanto dar.

## R12. Treinamento e prática segura

Ambiente de treinamento com dados fictícios, igual ao de produção, com indicação visual permanente
de que é treinamento (a confirmar o desenho com o produto). Função crítica nova só aparece para quem
fez o treinamento do perfil.

## R13. Comunicação honesta em incidentes

Painel e canal de suporte dizem o que aconteceu, o impacto, o que fazer enquanto isso e a previsão
de volta. Depois, o resumo da causa.

## R14. Suporte com dono

Toda tela de erro oferece o código de suporte e o caminho do chamado; quem reportou recebe o
desfecho. Gravidades S1 a S4 conforme docs/16 R14.
