# Playbook · Auditoria de segurança: cliente, segredos e banco

> Uso: `/seguranca`. Regras em [CLAUDE.md](../../CLAUDE.md) e [docs/15](../15-padrao-de-qualidade.md).

```text
Você é um auditor de segurança. Parta do princípio de que qualquer coisa que roda no navegador ou no Desktop (Tauri) pode ser lida e alterada pelo usuário. Quero uma auditoria completa e um plano de correção, seguindo o G1 do CLAUDE.md.

1. Regras de negócio na tela: liste todo cálculo de preço, desconto, cortesia, valor de guia, permissão, perfil, limite ou validação que só existe em apps/web, apps/portal ou apps/desktop. Para cada um, proponha mover a regra para o núcleo (rota + caso de uso no domínio) e fazer o servidor recalcular tudo, ignorando valores vindos do cliente.
2. Segredos: procure chaves e tokens no código de tela, em arquivos estáticos, na configuração do Tauri e em variáveis que vão para o navegador (VITE_*). Classifique cada uma como pública por design (URL do servidor, ID público do cliente OIDC, chave pública) ou secreta (credenciais do PSP do PIX, token da Meta, certificado e senha de assinatura, credencial da NFS-e, senha do banco, chave de API de IA). Toda secreta sai da tela e vai para o servidor (cofre SOPS/age). Diga quais preciso rotacionar porque já foram publicadas (inclusive no histórico do git).
3. Banco:
   a. confirme que o PostgreSQL só é acessível pelo núcleo e que cada módulo usa um papel com o mínimo de privilégio;
   b. no portal: liste as tabelas e diga quais estão com RLS desligado; para cada policy, explique em português quem pode ler, inserir, alterar e apagar o quê;
   c. aponte policies permissivas demais, como USING (true), WITH CHECK (true) ou regras que não comparam com o paciente autenticado;
   d. proponha policies mínimas: cada paciente só acessa as próprias linhas, e campos sensíveis (status de pagamento, papel, publicação do laudo) só mudam pelo servidor;
   e. confirme que nenhuma credencial administrativa do banco ou dos provedores é usada na tela.
4. Gere um relatório final com: achado, risco (alto, médio, baixo), arquivo e linha, correção proposta.

Regras: primeiro só o relatório, sem alterar nada. Depois aplique as correções uma por vez, com migration separada para as policies e papéis, esperando meu OK em cada uma. Não desligue nenhuma proteção existente para "fazer funcionar".
```
