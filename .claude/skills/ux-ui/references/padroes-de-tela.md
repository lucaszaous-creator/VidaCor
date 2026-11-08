# Padrões de tela

Fontes: `docs/05-design-system.md` (Regras de acabamento), `docs/19-menu-de-contexto.md`,
`docs/18-perfis-permissoes-e-rls.md`, mockups `Pacientes-Lista`, `Estados`, `Menus-Contexto`,
`Recepcao-Interativa` e `Barra-Paciente` em `design/mockups/`.

## 1. Estrutura padrão

```
Cabeçalho de uma linha: título (e contador) | busca | ação principal
Abas da tela (se houver)
Área de trabalho com 1 a 3 painéis (.fill, .card.pane, .scroll)
  painel principal: lista ou formulário, rola por dentro com cabeçalho fixo
  painel de detalhe (opcional): o item selecionado
```

- Modelo: `design/mockups/Pacientes-Lista.dc.html` (lista à esquerda, detalhe de 320 px à direita).
- O secundário vai para **abas**, **painel de detalhe** ou **gaveta**; nunca empilhado para baixo.
- Tela clínica: barra do paciente no topo, acima do cabeçalho da tarefa.

## 2. A tela cabe na janela

- Desenhada para 1440×900 e funcionando em 1280×800 **sem rolar a página**.
- O que é longo rola dentro do próprio painel (lista, tabela, conversa, linha do tempo).
- Menu lateral rola por dentro; só o grupo da tela atual aberto.
- Exceções com tamanho próprio: apps 390×844, tablet 1180×820, telão 1280×720, impressos A4.
- Pensar também em 1366×768 e zoom de 150% (docs/05, Acessibilidade): nesses casos o conteúdo
  reflui (grades `.g2`/`.g3` quebram), sem cortar ação nem identificador.

## 3. Sem rolagem lateral

- A partir de 1280 px (menu de 280 px mais 1000 px de conteúdo) nada obriga a arrastar para o lado.
- Tabela cabe na largura: menos colunas (junte atributos numa célula, como nome com nascimento e
  idade), texto quebrando, detalhe abaixo ou no painel em vez de coluna extra.
- Proibido `min-width` fixo em tabela e quadro.
- Exceções: telão e visualizador de imagens.

## 4. Estados de tela

Todo bloco de dados tem estes estados desenhados (catálogo em `Estados.dc.html`):

| Estado | O que a tela mostra |
|---|---|
| Vazio | O que é este bloco e a ação que o preenche ("Nenhum exame hoje. Agendar") |
| Carregando | Esqueleto com a forma do conteúdo (único gradiente permitido) |
| Sem conexão | Aviso fixo; o que foi digitado continua; reenvio ao reconectar (R6) |
| Erro | Mensagem R8 com código de suporte; nunca "0" ou lista vazia no lugar do erro |
| Sem permissão | O que não pode ver e a quem pedir; sem revelar se o objeto existe (SEC-DAD-4) |
| Salvamento automático | Indicação discreta de "rascunho salvo às 10:42" |
| Conflito | Outra pessoa alterou: mostrar as duas versões e deixar escolher, sem perder nada |
| Feito com desfazer | Confirmação curta com "Desfazer" por alguns segundos (R5) |
| Ato irreversível | Confirmação com o quê e com quem (R3) |
| Campo com erro | Formato explicado antes; erro junto do campo, em texto, com o valor mantido |
| Busca sem resultado | O termo buscado e o próximo passo (cadastrar, ampliar filtro) |
| Avisos | Bloqueante, atenção e informativo (R4), e a central de avisos |

## 5. Telas clínicas

- Dois identificadores (nome completo e nascimento) visíveis no topo e em todo impresso (R2).
- Alerta de homônimo quando há nome parecido no mesmo dia.
- Ato clínico (check-in, sala, conciliação, laudo, prescrição, entrega) só com a confirmação dos
  dois identificadores; pulseira bipada na sala errada **bloqueia** o exame (docs/20).
- Imagem sem paciente conciliado nunca abre no laudo (R1).
- Alergia sempre visível em vermelho cheio, nunca cortada.

## 6. O que aparece depende do perfil

- A tela mostra só o nível de dado que o perfil pode ver (docs/18 § 4): N0 operacional, N1
  cadastral, N2 financeiro, N3 clínico resumido, N4 clínico completo.
- Listas e esteira mostram só N0 e N1 mínimos; N3 e N4 só na tela do ato (docs/18 § 6.5).
- TV: só senha, primeiro nome com iniciais e sala (docs/18 § 6.8).
- Médico fora do vínculo de cuidado vê o caminho de **acesso de emergência**, com justificativa
  obrigatória e aviso de que será revisado (docs/18 § 6.2).
- Desenhar a mesma tela para pelo menos dois perfis quando o conteúdo muda (ex.: recepção e médico
  na ficha do paciente).
- A decisão é do servidor; a tela só reflete. Nunca desenhar "regra" na tela (G1).

## 7. Verificação automática dos mockups

Rodar antes de entregar. Requer `node` e `playwright` (já instalados no ambiente remoto).

```bash
tools/verificacao/verifica.sh Pacientes-Lista.dc.html
```

Renderiza com o motor do canvas em 1440×900 e 1280×800 e acusa `sh` (altura da página maior que a
janela), `cortado` (painel escondendo conteúdo) e `ladoalado` (rolagem lateral). Capturas em
`tools/verificacao/shots/`. Requer o motor do canvas em `tools/verificacao/render/support.js`, que
não é versionado; sem ele, registrar "não verificado" e o motivo.

```bash
node tools/verificacao/sem-rolagem-lateral.js "$PWD/design/mockups" design/vc10.css 1000
```

Mede a largura de todas as telas com 1000 px de conteúdo e lista as que rolam para o lado.

```bash
python3 tools/verificacao/dados_ficticios.py
```

Falha quando um paciente aparece com nascimentos diferentes ou a idade não bate com 07/11/2026.

## 8. Menu de contexto

Regras completas em `docs/19-menu-de-contexto.md`; protótipo em `Recepcao-Interativa.dc.html`.

- **Abrir:** clique direito, tecla Menu ou Shift+F10, e o botão de mais opções em cada objeto.
- **Cabeçalho** com o objeto: paciente com nome e nascimento; agendamento com horário e exame;
  pagamento com valor e forma.
- **Ordem fixa:** abrir, ações do dia a dia, comunicação, impressão, destrutivas no fim, em
  vermelho, separadas por linha.
- No máximo 12 itens por nível e 1 nível de submenu; submenus com sugestões calculadas pelo
  servidor, escolhíveis por número.
- **Permissão:** ação que o perfil nunca terá não aparece; indisponível agora aparece apagada com o
  motivo ("Laudo ainda não assinado"); o servidor confere de novo ao executar.
- Destrutivas pedem motivo e confirmação com o nome do paciente; estorno, assinatura, exportação e
  mudança de permissão pedem segundo fator.
- Registro clínico não tem desfazer: o menu mostra "Retificar (nova versão)".
- Catálogo mínimo por objeto: docs/19 § 7 (paciente, esteira, agendamento, horário livre, laudo na
  fila, laudo assinado, imagem, recebimento, texto clínico, aparelho, guia, item de estoque).

## 9. Seleção múltipla

- Ctrl ou Shift com clique; o cabeçalho diz "3 pacientes selecionados".
- Só ações seguras em lote: enviar preparo, confirmar presença, imprimir etiquetas, atribuir laudos.
- **Nada clínico em lote:** assinar, liberar e retificar são sempre um por um.

## 10. Impressos e etiquetas

- Impressos A4 (laudo, preparo, atestado, recibo) com nome e nascimento; modelos em
  `Laudo-Impresso`, `Preparo-Exame`, `Documentos`.
- Etiqueta só com nome, nascimento e códigos, nunca diagnóstico; conteúdo por tipo conforme
  DEC-2026-11-08-03 e `docs/20-etiquetas-e-rastreio.md`; modelos em `Etiquetas-Modelos`.
- Reimpressão pede motivo e mostra que a anterior foi invalidada.
