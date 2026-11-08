# Release, distribuição e gestão de mudança

Fontes: `docs/13-instalacao-e-distribuicao.md`, `docs/16-protecao-do-usuario.md` C5 e R9, `docs/14-regras-de-arquitetura-e-engenharia.md`
A6, `docs/modelos/prontidao-para-producao.md`, ADR-0013.

## 1. O que é publicado

| Artefato | Destino | Forma |
|---|---|---|
| Imagens do servidor (núcleo, integrações, gateway DICOM, assinatura, IA) | Servidor principal e de contingência | Contêineres assinados, por digest |
| VidaCor Desktop | Estações Windows e TV | MSI assinado; atualização pelo updater do Tauri a partir do servidor local |
| Aplicação web | Navegadores da rede | Servida pelo núcleo ou pelo Caddy, mesma versão do servidor |
| Portal | Nuvem no Brasil | Publicação separada, com o mesmo controle de assinatura e prontidão |
| SBOM e notas de versão | Repositório de versões | Por versão |

## 2. Canais

- `piloto`: uma ou duas máquinas ou salas escolhidas recebem antes (docs/13 §4, docs/16 C5).
- `estavel`: produção, depois do piloto aprovado.
- Compatibilidade: Desktop e servidor negociam a versão da API; estação desatualizada funciona por prazo
  definido (docs/14 A6).

## 3. Caminho de uma mudança até a produção

1. **Prontidão** completa (§ 7) e assinada pelo dono do produto, dono técnico e responsável clínico
   quando clínico.
2. **Janela** fora do horário de pico (PREM-23: segunda a sexta 07:00 às 19:00, sábado 07:00 às 13:00, a
   confirmar), com o dono técnico disponível.
3. **Congelamento** conferido: nada em véspera de feriado, fechamento do faturamento ou com incidente
   aberto. Exceção só para correção de S1 ou de segurança, com registro.
4. **Backup** imediatamente antes; verificação de que o backup é restaurável.
5. **Migração** aditiva (expand/contract); nenhuma remoção de coluna ou tabela na mesma versão que
   deixa de usá-la. Migration revisada por pessoa (G6).
6. **Uso interno primeiro** em `homolog` com dado fictício (docs/16 C5).
7. **Piloto** com comparação automática de erros e latência contra a versão atual (análise canário).
   Piora além do critério definido: volta sozinha.
8. **Liberação gradual por chave liga/desliga**: por perfil, máquina ou sala, nunca a clínica inteira de
   uma vez.
9. **Acompanhamento**: SLO e erros observados durante a janela; decisão de seguir ou voltar registrada.
10. **Comunicado**: mudança visível avisada antes à equipe, com o que mudou e por quê (docs/16 R9);
    treinamento feito quando a função é crítica (R12).

## 4. Critério de volta automática (esboço a calibrar)

| Sinal no piloto, comparado à versão atual | Ação |
|---|---|
| Taxa de erro 5xx acima de 2 vezes a referência por 10 min | Volta automática |
| p95 das rotas operacionais acima de 150 ms por 15 min | Volta automática |
| Falha de health check em qualquer serviço | Volta automática |
| Estudos sem conciliação aumentando | Pausa e avaliação com o dono técnico |

Os números finais ficam no ADR de publicação, **a confirmar** com medição em `homolog`.

## 5. Volta à versão anterior

- Um comando volta servidor e estações para a versão anterior (docs/13 §4). O comando é testado **nesta
  versão**, em `homolog`, antes da janela.
- Migração aditiva garante que a versão anterior funciona com o banco novo. Se uma mudança não permitir
  isso, o PR precisa de plano de contorno documentado e aprovação explícita (docs/14 B2 item 6).
- A volta nunca apaga dado: registros criados na versão nova continuam no banco.

## 6. Chaves liga/desliga

- Toda função nova visível ao usuário nasce desligada, com chave por perfil, máquina, sala ou unidade
  (docs/14 A6).
- A chave é configuração versionada (com quem mudou, quando e por quê, docs/21 §3.16), não variável solta.
- Chave antiga é removida em PR próprio depois que a função está em `estavel` para todos.
- Chave nunca substitui permissão: o servidor continua conferindo o ato (G1).

## 7. Checklist de release (modelo resumido de `docs/modelos/prontidao-para-producao.md`)

**Produto e clínica**
- [ ] Documento de design aprovado; riscos clínicos com controles testados; parecer clínico quando clínico
- [ ] Teste de usabilidade das tarefas críticas aprovado; treinamento e manual publicados

**Qualidade**
- [ ] G1 a G6 e checklist de proteção do usuário completos
- [ ] `just test` verde; falhas por bug real registradas em `docs/bugs.md`
- [ ] Carga dentro do orçamento p95 e p99; caos leve em `homolog` sem perda de dados

**Operação**
- [ ] Métricas, logs, traces, painéis e alertas prontos e testados
- [ ] Runbook atualizado; capacidade verificada
- [ ] Backup antes; volta testada; chave e canal piloto configurados
- [ ] Janela definida fora de congelamento; dono técnico disponível

**Segurança**
- [ ] Modelagem de ameaças revisada; varreduras limpas; permissões por ato revisadas
- [ ] Artefato assinado, mesmo digest do construído, SBOM publicado

Assinaturas: dono do produto, dono técnico, responsável clínico (quando clínico), data.

## 8. Notas de versão

Geradas do changelog (Conventional Commits) e revisadas por pessoa. Duas versões:
- **Técnica** para a TI: serviços alterados, migrações, novos alertas, ações manuais (de preferência
  nenhuma), como voltar.
- **Para a equipe da clínica**: o que mudou na tela e por quê, em português da clínica, sem jargão.

## 9. Anti-padrões de release

- Publicar por tag móvel em vez de digest.
- Migração que remove coluna junto com a mudança de código.
- Liberar para todas as salas de uma vez.
- Pular o piloto porque "é só uma correção pequena".
- Ligar uma função crítica para quem não fez o treinamento.
