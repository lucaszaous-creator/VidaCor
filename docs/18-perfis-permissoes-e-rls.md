# 18 · Perfis, permissões e RLS: quem vê e faz o quê

> **Status:** proposta para validação com a direção e o responsável clínico de
> segurança (v0.1, nov/2026). Decisão no [ADR-0017](adr/0017-permissoes-e-rls.md).
> Liga-se a G1 ([docs/15](15-padrao-de-qualidade.md)), SEC-ACS e SEC-DAD
> ([docs/17](17-seguranca-por-classe-de-ataque.md)) e à LGPD ([docs/08](08-seguranca-lgpd-etica.md)).

## 1. Modelo em três camadas

| Camada | O que faz | Onde |
|---|---|---|
| **1. Perfil (RBAC)** | Define **quais atos** a pessoa pode fazer (ex.: `laudo.assinar`, `caixa.estornar`) | Keycloak (papéis) + núcleo (catálogo de atos) |
| **2. Contexto (ABAC)** | Define **sobre quais objetos**: unidade, sala, vínculo com o paciente, laudo atribuído, caixa próprio, horário | Núcleo, em cada caso de uso (SEC-ACS-2) |
| **3. RLS no PostgreSQL** | **Segunda barreira**: mesmo se o código errar, o banco não entrega linha fora do escopo | Políticas por tabela, com o contexto da transação |

A decisão é sempre do **servidor**. A tela só esconde o que o usuário não pode
fazer, como conforto (G1).

## 2. Níveis de dado

| Nível | Exemplos | Sensibilidade |
|---|---|---|
| **N0 · Operacional** | Horário, sala, status na esteira, senha | Baixa |
| **N1 · Cadastral** | Nome, nascimento, contato, convênio, carteirinha | Pessoal |
| **N2 · Administrativo-financeiro** | Guias, valores, pagamentos, glosas, NF | Pessoal + financeiro |
| **N3 · Clínico resumido** | Exame pedido, preparo, alergias, alertas clínicos, status do laudo | **Saúde (sensível)** |
| **N4 · Clínico completo** | Laudo, evolução, prescrição, imagens, anamnese, documentos clínicos | **Saúde (sensível)** |

## 3. Perfis

| Perfil | Quem é |
|---|---|
| **Recepção** | Balcão: agenda, check-in, senhas, documentos |
| **Caixa** | Recebimentos do particular (normalmente combinado com Recepção) |
| **Técnico de sala / enfermagem** | Executa o exame (USG, RX, TC, MAPA, ergométrico) |
| **Médico executante/laudador** | Faz o exame e/ou o laudo |
| **Médico revisor** | Segunda leitura de laudos (quando houver) |
| **Faturista** | Guias, lotes TISS, glosas |
| **Financeiro** | Contas, conciliação, DRE, repasse, NF |
| **Estoque/compras** | Itens, lotes, pedidos |
| **Atendimento WhatsApp** | Conversas e agendamento pelo WhatsApp |
| **Gerente/direção** | Visão da operação e indicadores |
| **Responsável clínico de segurança** | Médico que aprova fluxos clínicos e investiga incidentes clínicos |
| **Encarregado LGPD (DPO) / auditor** | Auditoria e pedidos de titulares |
| **TI/administrador** | Usuários, aparelhos, integrações, backup |
| **Paciente** (portal/app) | O próprio paciente ou responsável legal |
| **Médico solicitante externo** (portal) | Médico que pediu o exame, por link compartilhado |
| **Dispositivo TV** (`tv`) | Só o painel de chamada; conta sem outro perfil (DEC-2026-11-08-73) |

## 4. O que cada perfil **vê**

**Sim**: vê; **Parcial**: vê parte (regra ao lado); **—**: não vê

| Perfil | N0 Operacional | N1 Cadastral | N2 Financeiro | N3 Clínico resumido | N4 Clínico completo |
|---|---|---|---|---|---|
| Recepção | Sim | Sim | Parcial: guia e pendência do paciente atendido | Parcial: preparo, alergias, alertas para o atendimento | — |
| Caixa | Sim | Sim | Parcial: só recebimentos e orçamentos do próprio caixa | — | — |
| Técnico de sala | Parcial: da sua sala | Parcial: dos pacientes da sua sala hoje | — | Sim dos pacientes da sua sala | Parcial: imagens do exame que executa; laudo final só depois de liberado |
| Médico executante/laudador | Sim | Sim dos seus pacientes | — | Sim dos seus pacientes | Parcial: **com vínculo**: agendado com ele, em exame, laudo atribuído ou histórico do paciente em atendimento |
| Médico revisor | Parcial: | Parcial: | — | Sim dos laudos para revisão | Parcial: laudos atribuídos para revisão |
| Faturista | Parcial: | Sim | Sim | Parcial: só códigos TUSS/CID exigidos na guia | — |
| Financeiro | — | Parcial: nome e documento fiscal | Sim | — | — |
| Estoque/compras | — | — | Parcial: custos e fornecedores | — | — (consumo de contraste por exame sem identificar paciente) |
| Atendimento WhatsApp | Sim | Sim | Parcial: valores de orçamento | Parcial: preparo e status do laudo (sem conteúdo) | — |
| Gerente/direção | Sim | Parcial: | Sim | Parcial: agregados e indicadores | — (sem acesso individual por padrão) |
| Responsável clínico de segurança | Sim | Sim | — | Sim | Parcial: em investigação registrada |
| DPO/auditor | Parcial: | Parcial: | Parcial: | Parcial: metadados de acesso (quem viu o quê, quando) | — (conteúdo só com ordem justificada) |
| TI/administrador | Parcial: técnico | — | — | — | — (**zero dado clínico** por padrão) |
| Paciente | Parcial: seus agendamentos | Sim o próprio | Sim seus recibos e notas | Sim o próprio | Sim **só laudos assinados e liberados** e suas imagens |
| Médico solicitante externo | — | Parcial: do paciente compartilhado | — | Parcial: do exame compartilhado | Parcial: só o laudo e as imagens compartilhados, até o link expirar |
| Dispositivo TV | Parcial: senha, nome abreviado, sala | — | — | — | — |

## 5. O que cada perfil **faz** (atos principais)

| Ato | Recep. | Caixa | Técnico | Médico | Revisor | Fatur. | Financ. | Gerente | TI | Paciente |
|---|---|---|---|---|---|---|---|---|---|---|
| Agendar, encaixar, remarcar | Sim | — | — | Parcial: bloquear a própria agenda | — | — | — | Sim | — | Parcial: agendamento online |
| Check-in, documentos, etiquetas | Sim | — | — | — | — | — | — | — | — | Parcial: pré-check-in |
| Chamar paciente (TV) | Sim | — | Sim | Sim | — | — | — | — | — | — |
| Abrir/fechar caixa, receber | — | Sim | — | — | — | — | — | Parcial: conferir fechamento | — | Parcial: pagar PIX |
| Desconto acima do limite | — | Parcial: pede | — | — | — | — | — | Sim aprova (**step-up**) | — | — |
| Estornar | — | Parcial: pede | — | — | — | — | Sim (**step-up**) | Sim (**step-up**) | — | — |
| Executar exame, registrar contraste | — | — | Sim | Sim | — | — | — | — | — | — |
| Vincular imagem manualmente | — | — | Parcial: propõe | Sim (**dois identificadores**) | — | — | — | — | — | — |
| Escrever evolução/laudo | — | — | — | Sim | Parcial: comentar | — | — | — | — | — |
| **Assinar e liberar laudo** | — | — | — | Sim (**step-up**) | Parcial: coassinar | — | — | — | — | — |
| Retificar laudo (nova versão) | — | — | — | Sim autor ou responsável técnico | — | — | — | — | — | — |
| Marcar "conversar antes" | — | — | — | Sim | Sim | — | — | — | — | — |
| Prescrever, atestado, pedido | — | — | — | Sim | — | — | — | — | — | — |
| Gerar guia, lote TISS, recurso | — | — | — | — | — | Sim | — | Parcial: | — | — |
| Contas, conciliação, repasse, NF | — | — | — | — | — | — | Sim | Sim | — | — |
| Exportar dados | — | — | — | — | — | Parcial: faturamento | Parcial: financeiro | Parcial: agregados (**step-up**) | — | Sim os próprios |
| Gerenciar usuários e permissões | — | — | — | — | — | — | — | Parcial: aprova | Sim (**step-up**, duas pessoas para perfil clínico) | — |
| Cadastrar aparelhos, integrações, backup | — | — | — | — | — | — | — | — | Sim | — |
| Compartilhar laudo com médico externo | — | — | — | Sim | — | — | — | — | — | Sim |
| Revogar consentimento, pedir exclusão | — | — | — | — | — | — | — | — | — | Sim |

**Step-up** = pedir de novo o segundo fator na hora do ato (SEC-AUT-5).

## 6. Regras especiais

1. **Vínculo de cuidado:** o médico acessa N4 de um paciente só com vínculo (agendamento com ele, exame em execução, laudo atribuído, atendimento aberto). O vínculo **expira** (ex.: 30 dias após o último ato), exceto para laudos que ele assinou.
2. **Acesso de emergência (*break-glass*):** fora do vínculo, o médico pode acessar com **justificativa obrigatória**. O acesso é registrado em destaque, avisado ao responsável clínico de segurança e revisado em até 48 h.
3. **Separação de funções:** quem pede o estorno não aprova o estorno; quem cria usuário com perfil clínico precisa da aprovação de uma segunda pessoa; a TI não tem dado clínico.
4. **Escopo de unidade:** se a VidaCor tiver mais de uma unidade, todo dado carrega a unidade e o filtro vale em todas as camadas.
5. **Menor dado necessário:** listas e esteira mostram só N0/N1 mínimos; N3/N4 só na tela do ato.
6. **Paciente menor ou incapaz:** o portal é do responsável legal, com vínculo registrado; quando o paciente atinge a maioridade, o acesso passa para ele.
7. **Funcionário atendido como paciente:** prontuário de colega tem alerta e acesso ainda mais restrito (só vínculo direto), com auditoria destacada.
8. **TV:** só senha, primeiro nome e iniciais, e sala.
9. **Tudo auditado:** toda leitura de N3/N4 e todo ato sensível gravam quem, o quê, quando, de onde e por quê (quando há justificativa).

## 7. Como o RLS funciona no VidaCor

O núcleo é o único cliente do banco. A cada requisição, abre uma transação e
**declara o contexto** do usuário; as políticas RLS usam esse contexto.

```sql
-- No início de cada transação (feito pelo núcleo, nunca pela tela)
SET LOCAL app.usuario_id   = '<uuid>';
SET LOCAL app.perfis       = 'medico';          -- lista de perfis
SET LOCAL app.unidade_id   = '<uuid>';
SET LOCAL app.break_glass  = 'false';

-- Tabelas com dado clínico têm RLS ligado e forçado
ALTER TABLE laudos.laudo ENABLE ROW LEVEL SECURITY;
ALTER TABLE laudos.laudo FORCE ROW LEVEL SECURITY;

-- Exemplo: médico lê laudo só com vínculo de cuidado (ou break-glass registrado)
CREATE POLICY laudo_leitura_medico ON laudos.laudo
  FOR SELECT
  USING (
    current_setting('app.perfis') LIKE '%medico%'
    AND unidade_id = current_setting('app.unidade_id')::uuid
    AND (
      EXISTS (SELECT 1 FROM cuidado.vinculo v
               WHERE v.paciente_id = laudo.paciente_id
                 AND v.medico_id   = current_setting('app.usuario_id')::uuid
                 AND v.expira_em   > now())
      OR current_setting('app.break_glass') = 'true'
    )
  );

-- Laudo assinado é imutável: nenhuma política permite UPDATE/DELETE
-- (correção = INSERT de nova versão)
```

Regras do RLS:
- **Ligado e forçado** (`FORCE ROW LEVEL SECURITY`) em toda tabela com N3/N4 e em tabelas financeiras.
- **Um papel de banco por módulo**, sem `BYPASSRLS`. Só migrações usam papel administrativo, fora do horário e auditadas.
- **Proibido** `USING (true)` / `WITH CHECK (true)` **[CI: lint de migração]**.
- O contexto é declarado com `SET LOCAL` (vale só na transação), nunca com `SET` de sessão (evita vazar contexto entre requisições no pool).
- **Teste de RLS obrigatório**: para cada política, um teste com usuário autorizado (vê) e um não autorizado (não vê), direto no banco e pela rota (G4).
- **Portal (nuvem):** RLS por `paciente_id` (e por link compartilhado com validade) no banco do portal.
- Leituras derivadas (Indicadores, busca, exportação) usam visões que **respeitam** o mesmo escopo (SEC-DAD-2).

## 8. Onde isso aparece no mockup

- **Configurações › Usuários e perfis**: matriz de permissões por ato e 2FA.
- **Minha agenda (médico)**: aviso "O que o médico vê" (vínculo e acesso de emergência).
- **Ficha do paciente › Auditoria**: quem acessou e por quê.
- **Portal**: o paciente vê quem acessou seus dados (R11 do docs/16).

## 9. Pendente de validação com a clínica
- Lista final de perfis (por exemplo, se Recepção e Caixa são sempre a mesma pessoa).
- Prazo do vínculo de cuidado e quem revisa o acesso de emergência.
- Se haverá médico revisor e segunda leitura.
- Limites de desconto por perfil.
- Unidades (uma ou várias).
