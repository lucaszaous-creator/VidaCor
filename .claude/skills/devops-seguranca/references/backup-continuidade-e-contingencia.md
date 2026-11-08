# Backup, continuidade e apoio à contingência

Fontes: `docs/11-arquitetura-e-infraestrutura.md` §7.2 e §7.4, `docs/16-protecao-do-usuario.md` C4 e C6,
`docs/23-modo-contingencia.md`, `docs/25-plano-do-projeto.md` §7.1, `docs/17-seguranca-por-classe-de-ataque.md` SEC-DAD-5.

## 1. Metas

| Métrica | Meta |
|---|---|
| RPO (perda máxima de dados) | 5 min, por réplica e WAL contínuo |
| RTO (volta com o servidor de contingência) | 30 min |
| Restauração completa testada | Mensal, com relatório (docs/25 §7.1) |
| Troca para o servidor de contingência | Exercício trimestral (docs/16 C6) |
| Incidente de segurança | Exercício semestral |

## 2. O que é copiado e como

| Dado | Ferramenta | Destino | Observação |
|---|---|---|---|
| PostgreSQL | Réplica por streaming; `pgBackRest` com PITR (backup completo e incremental, WAL arquivado) | Servidor de contingência e nuvem no Brasil | Criptografado no cliente |
| Imagens (Orthanc) | Réplica para a contingência; `restic` para o destino externo | Contingência e nuvem | Hash conferido na chegada e periodicamente (docs/21 §5) |
| Documentos e arquivos (PDF, anexos, kit) | `restic` | Nuvem | Criptografado no cliente |
| Configuração e segredos | Repositório de infraestrutura (SOPS) e cópia cifrada da chave de recuperação | Fora do servidor | Procedimento da chave de recuperação a confirmar com a clínica |
| Snapshots ZFS | Locais, frequentes | Mesmo servidor | Proteção rápida; não substitui o backup externo |

Regra 3-2-1: três cópias, em dois tipos de mídia, uma fora da clínica. A cópia externa precisa resistir a
ransomware: credencial de escrita sem permissão de apagar, retenção imutável quando o provedor permitir
(a confirmar com o provedor escolhido).

Retenção de backup segue a retenção do dado (docs/21 §5 item 7): prontuário e imagens por 20 anos; o
plano de camadas (online e fria) é **a confirmar** com a clínica (docs/11 §7.1).

## 3. Segurança dos backups (SEC-DAD-5)

- Criptografados antes de sair do servidor; chaves no cofre, separadas do destino.
- Acesso só à operação designada; TI sem dado clínico também no backup: restauração para teste ocorre em
  ambiente isolado e o conteúdo clínico não é aberto.
- Restauração não devolve acesso revogado nem estado inválido: depois de restaurar, revogações e sessões
  são reaplicadas a partir do Keycloak e da auditoria.
- Backup nunca é copiado para `dev` ou `homolog`.

## 4. Exercício mensal de restauração

Verificação (esboço; comandos a confirmar com a versão das ferramentas):

```bash
# 1. Restaurar o PostgreSQL num ambiente isolado, até um ponto no tempo
pgbackrest --stanza=vidacor --type=time --target="<AAAA-MM-DD HH:MM:SS-03>" \
  --pg1-path=/srv/restauracao/pg restore

# 2. Subir o banco restaurado sem rede externa e conferir integridade
psql -h /srv/restauracao/run -d vidacor -c "select count(*) from auditoria.evento;"
# conferir a cadeia de hash da auditoria com a ferramenta do próprio núcleo
/app/nucleo verificar-auditoria --banco /srv/restauracao/run

# 3. Restaurar uma amostra de imagens e conferir o hash
restic -r <repositorio> restore latest --target /srv/restauracao/imagens --include "<amostra>"
/app/nucleo verificar-hash-imagens --pasta /srv/restauracao/imagens --amostra 200

# 4. Medir o tempo total e destruir o ambiente de restauração, com registro
```

Relatório de restauração (template):

```markdown
# Relatório de restauração: <AAAA-MM>
Executado por: <nome>  Data: <data>  Ambiente: <isolado, sem rede externa>

| Item | Resultado |
|---|---|
| Ponto no tempo pedido e obtido | |
| Tempo total até o banco utilizável | |
| Cadeia de hash da auditoria | íntegra / divergente (detalhar) |
| Amostra de imagens com hash conferido | <n> de <n> |
| Documentos restaurados e abertos | <n> de <n> |
| Problemas encontrados | |
| Ações (dono, prazo) | |
| Ambiente de restauração destruído em | |
```

Falha no exercício é incidente S2 de operação e gera ação com dono e prazo.

## 5. Troca para o servidor de contingência (trimestral)

1. Agendado fora do horário, em conjunto com o simulado de contingência do trimestre (docs/23 §10).
2. Promover a réplica do PostgreSQL, apontar o gateway para a contingência, conferir Orthanc e NATS.
3. Medir RTO e conferir que nenhuma transação confirmada se perdeu além do RPO.
4. Voltar para o principal e reconstruir a réplica.
5. Relatório no modelo de pós-incidente, com ações.

## 6. Apoio técnico ao modo contingência (docs/23)

A TI **não declara** contingência sozinha: informa diagnóstico e estimativa; quem declara é a
coordenação, a recepção líder na ausência dela, ou o médico por motivo clínico (docs/23 §4).

| Nível | Situação | Papel técnico |
|---|---|---|
| N1 externo | Internet fora | Registrar ocorrência de integração; confirmar que o outbox acumula e reenvia; avisar se passar de 4 h em dia útil e travar autorizações |
| N2 setor | Estação, impressora ou trecho de rede fora | Diagnóstico, troca de estação, previsão |
| N3 total | Servidor, banco ou rede interna fora; energia além do nobreak | Diagnóstico, troca para a contingência (RTO 30 min), estimativa de volta |

Gatilhos que a TI informa de imediato à coordenação: núcleo sem resposta em duas estações de setores
diferentes por mais de 10 min; estimativa de volta acima de 15 min; nobreak com menos de 10 min de
autonomia (docs/23 §3).

Infraestrutura do kit das 06:00 (docs/23 §5 e §6):
- Tarefa agendada às 06:00 gera o kit em PDF (Typst) com versão e hash no lastro; alerta à TI e à
  coordenação se falhar até 06:15.
- Cópia local cifrada nas estações de recepção e salas, somente leitura, válida só no dia, atualizada a
  cada 15 min; abertura com login recente e PIN, registrada. Detalhes de cifra, chave e expiração
  dependem de **ADR pendente** (CONT-21, DEC-2026-11-08-14).
- Faixas de numeração reservadas no banco com índice único (G2).

Retomada técnica (docs/23 §8.1): TI confirma servidor, banco, rede e integrações e registra a hora da
volta; estações enviam o que gravaram localmente, com idempotência; conferência técnica do lançamento e
da conciliação das imagens. O encerramento é da coordenação, com reautenticação.

## 7. Caos leve em homologação (docs/16 C4)

A cada versão maior, derrubar de propósito banco, PACS, NATS, internet e um provedor externo em
`homolog`, e provar que o atendimento segue e nada se perde. Nunca em produção.
