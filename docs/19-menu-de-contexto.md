# 19 · Menu de contexto (clique direito)

> **Status:** padrão de interface obrigatório (v0.1, nov/2026). Mockup:
> `design/mockups/Menus-Contexto.dc.html` (8 situações reais com menus e submenus abertos).
> Complementa o [design system](05-design-system.md), a [proteção do usuário](16-protecao-do-usuario.md)
> e as [permissões](18-perfis-permissoes-e-rls.md).

Cada **objeto** da tela (paciente, agendamento, horário livre, laudo, imagem,
pagamento, texto clínico) tem um menu com o que dá para fazer com ele ali mesmo,
sem trocar de tela.

## 1. Como abrir
| Forma | Uso |
|---|---|
| Clique direito | Mouse |
| Tecla **Menu** ou **Shift+F10** no item focado | Teclado e leitores de tela |
| Toque longo | Tablet |
| Botão **⋯** visível em cada item | Quem não usa clique direito (o menu nunca é a única forma de chegar a uma ação) |

## 2. Como navegar
- Setas ↑/↓ percorrem; → abre submenu; ← volta; **Enter** executa; **Esc** fecha e devolve o foco ao item.
- **Digitar filtra** os itens (campo de filtro no topo dos menus com mais de 8 itens).
- Atalho de teclado mostrado à direita de cada ação; o atalho funciona também com o menu fechado.
- Acessibilidade: `role="menu"`, `menuitem`, `menuitemcheckbox`, `aria-disabled`, foco preso no menu enquanto aberto, contraste AA também no tema escuro do visualizador.

## 3. Como organizar
1. **Cabeçalho** com o objeto: paciente sempre com **nome + nascimento** (dois identificadores); agendamento com horário e exame; pagamento com valor e forma.
2. Ordem fixa: **abrir** → ações do dia a dia → comunicação → impressão → **destrutivas no fim, em vermelho**, separadas por linha.
3. No máximo **12 itens por nível** e **1 nível de submenu**.
4. Submenus trazem **sugestões calculadas pelo servidor** (ex.: próximos horários livres que cabem no exame), escolhíveis por número (1, 2, 3).
5. Itens liga/desliga aparecem como caixa de marcar.

## 4. Permissão e estado
| Situação | Comportamento |
|---|---|
| O perfil **nunca** terá a ação | O item **não aparece** |
| A ação existe, mas está indisponível agora (estado, prazo, falta de permissão momentânea) | O item aparece **apagado, com o motivo** ("Laudo ainda não assinado", "Só supervisor da recepção") |
| Ação executada | O servidor **confere a permissão de novo** (G1); o menu é só conforto |

Os itens mudam com o **estado** do objeto (laudo pendente × assinado; horário ocupado × livre).

## 5. Segurança
- Destrutivas pedem **motivo** e confirmação com o **nome do paciente** (R3 do docs/16).
- Estorno, assinatura, exportação e mudança de permissão pedem **2º fator** (SEC-AUT-5); estorno exige **aprovação de outra pessoa**.
- Registro clínico não tem "desfazer": o menu mostra o caminho certo (Retificar, nova versão).
- Toda ação vai para a **auditoria**; repetir não duplica (G3).
- Exportar imagem sai **anonimizado** por padrão.

## 6. Seleção múltipla
- Ctrl/Shift + clique seleciona vários; o cabeçalho mostra "3 pacientes selecionados".
- Só ações **seguras em lote**: enviar preparo, confirmar presença, imprimir etiquetas, atribuir laudos.
- **Nada clínico em lote** (assinar, liberar, retificar).

## 7. Catálogo mínimo por objeto

| Objeto | Ações (resumo) |
|---|---|
| **Paciente** (em qualquer tela) | Abrir ficha · novo agendamento · copiar nome e nascimento · histórico · enviar acesso ao portal · imprimir etiqueta · unir duplicados (2 pessoas) · ver quem acessou (só DPO) |
| **Paciente na esteira** | Abrir ficha · check-in ▸ (etapas feitas e pendentes) · chamar na TV · mover para ▸ · preferencial · WhatsApp ▸ · receber no caixa · remarcar · etiqueta · desistência · cancelar (com motivo/permissão) |
| **Agendamento** | Abrir · confirmar presença · reenviar confirmação · remarcar ▸ (sugestões) · trocar médico ▸ · outra sala ▸ · recortar para mover · preparo ▸ · orçamento · marcar atraso · falta · cancelar |
| **Horário livre** | Agendar aqui · encaixar da lista de espera ▸ (só quem cabe) · colar agendamento · reservar para urgência · bloquear ▸ (motivo) · disponível no agendamento online (liga/desliga) |
| **Laudo na fila** | Abrir no laudo · abrir imagens · comparar com anterior · atribuir a ▸ (fila de cada médico) · prioridade ▸ · segunda leitura · "conversar antes de liberar" · enviar ao solicitante (após assinatura) · imprimir · compartilhar link seguro |
| **Laudo assinado** | Abrir · retificar (nova versão) · ver acessos — sem desfazer assinatura |
| **Imagem no visualizador** | Janela ▸ (predefinições por número) · medir ▸ · anotar · imagem-chave · sincronizar séries · comparar · girar/inverter · copiar para o laudo · exportar anonimizada · informações DICOM |
| **Recebimento no caixa** | Ver recibo · reenviar comprovante · ver NFS-e · copiar código PIX · ver na conciliação · observação · estornar (2º fator + aprovação) |
| **Texto clínico** (evolução/laudo) | Inserir frase ▸ · inserir medida do aparelho ▸ · inserir CID · ditar daqui · sugestões de escrita · copiar · colar sem formatação |
| **Aparelho** (tela Aparelhos) | Testar conexão · ver ao vivo · reenviar worklist · pausar recebimento · ver registro · abrir configuração (só TI) |
| **Guia/lote TISS** | Abrir · conferir · gerar lote · reenviar · ver demonstrativo · registrar recurso de glosa |
| **Item de estoque/lote** | Ver lotes · dar baixa manual (motivo) · rastrear pacientes que usaram o lote · pedido de compra |

## 8. Implementação (para quando houver código)
- Um único componente `MenuDeContexto` no design system, alimentado por uma **lista de ações** vinda do servidor para aquele objeto (id, rótulo, atalho, estado, motivo, perigo, submenu). A tela **não decide** quais ações existem.
- Cada ação chama a mesma rota da ação equivalente fora do menu (sem caminho paralelo).
- Testes: teclado (Shift+F10, setas, Esc), leitor de tela, item indisponível com motivo, item inexistente para o perfil, ação destrutiva com confirmação, seleção múltipla sem ação clínica.
