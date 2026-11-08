# Revisão de prontidão para produção — <módulo/versão>

Nada vai para produção sem todos os itens marcados (ou exceção aprovada e registrada).

## Produto e clínica
- [ ] Documento de design aprovado
- [ ] Riscos clínicos com controles implementados e testados; aprovação do responsável clínico de segurança
- [ ] Teste de usabilidade das tarefas críticas aprovado com a equipe
- [ ] Treinamento feito e manual publicado; mudança comunicada à equipe

## Qualidade
- [ ] Checklist G1–G6 e de proteção do usuário completos
- [ ] `just test` verde; nenhuma falha nova sem registro em `docs/bugs.md`
- [ ] Teste de carga dentro do orçamento p95/p99
- [ ] Exercício de caos em homologação sem perda de dados

## Operação
- [ ] Métricas, logs, traces e painéis prontos; alertas configurados e testados
- [ ] Manual de operação (reiniciar, diagnosticar, restaurar)
- [ ] Capacidade (disco, CPU, banco) verificada
- [ ] Backup antes do lançamento e volta à versão anterior testada
- [ ] Chave liga/desliga e canal piloto configurados
- [ ] Janela de lançamento definida; dono técnico disponível

## Segurança
- [ ] Modelagem de ameaças revisada; varreduras (SAST, dependências, segredos) limpas
- [ ] Permissões por ato revisadas; nenhum segredo na tela

**Aprovado por:** dono do produto [ ] · dono técnico [ ] · responsável clínico [ ] · data:
