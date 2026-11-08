# Mobile, portal responsivo e tablet

Fontes: `docs/25-plano-do-projeto.md` §2 (MVP) e §3.1 (Fase 5); PREM-24; `docs/04-escopo-funcional.md`;
`docs/05-design-system.md` (exceções de tamanho); ADR-0005 e ADR-0011; `docs/17-seguranca-por-classe-de-ataque.md`.

## 1. Estado

| Item | Situação |
|---|---|
| Portal do paciente (convite, laudos, imagens, documentos) | **MVP**, web responsivo, na nuvem no Brasil (ADR-0011) |
| Tablets de sala e de assinatura (1180×820) | **MVP**, pela aplicação web ou pelo Desktop, conforme o uso |
| App do paciente, App do médico, Meus Exames Bebê | **Depois do MVP** (Fase 5, PREM-24 a confirmar) |
| Telemedicina | Depois do MVP (CFM 2.314/2022) |
| Tecnologia do app nativo | **ADR pendente**, a decidir antes do início da Fase 5 |

Os mockups já existem para referência (`design/mockups/App-Paciente.dc.html`, `App-Medico.dc.html`, `App-Bebe.dc.html`),
em 390×844.

## 2. Decisão de tecnologia móvel: ADR pendente

Não há decisão. Pela regra do ADR-0019, quando chegar a hora o tech lead propõe e registra. Opções e critérios a usar:

| Opção | A favor | Contra |
|---|---|---|
| React Native com Expo | Reaproveita TypeScript, cliente gerado do OpenAPI e tokens do design system; ecossistema de acessibilidade e push maduro | Segunda base de componentes de interface; dependência de serviços do Expo a avaliar |
| PWA (portal instalável) | Uma base só; sem loja | Push e armazenamento seguro limitados em alguns sistemas; experiência offline restrita |
| Tauri mobile | Mesma casca do desktop | Maturidade a verificar na data da decisão |

Critérios: reaproveitamento do contrato e do design system, acessibilidade (WCAG 2.2 AA e letra ajustável), armazenamento
seguro, push sem conteúdo clínico, custo de manutenção com a equipe de sustentação (PREM-22), suporte da skill `e2e` aos
testes em iOS e Android, e requisitos de loja. Proposta inicial a confirmar no ADR: React Native com Expo, por reaproveitar
TypeScript e o cliente gerado.

## 3. Regras que valem qualquer que seja a tecnologia

- **Sem regra no cliente** (G1, G5): o app pede, o servidor decide; mesmo contrato OpenAPI do portal.
- **Nenhum segredo no app**: tudo que vai no pacote pode ser lido. Só valores públicos por design.
- **Autenticação**: OIDC com PKCE pelo navegador do sistema (SEC-AUT-1); contas de paciente separadas das da equipe
  (ADR-0010); token no armazenamento seguro do sistema (Keychain, Keystore), nunca em armazenamento comum.
- **Sem dado de saúde em cache** persistente, em captura de tela do alternador de apps (ocultar conteúdo ao ir para segundo
  plano) ou em log.
- **Push sem conteúdo clínico**: "Há um novo documento no seu portal", nunca resultado ou nome de exame sensível.
- Laudo só aparece **assinado e liberado**; resultado delicado só depois da conversa médica (R11).
- Links de imagem e PDF assinados, curtos e vinculados ao paciente (SEC-DAD-6).
- Paciente menor ou incapaz: acesso do responsável legal com vínculo registrado (`docs/18` §6 item 6).
- Pinagem de certificado: **a avaliar** no ADR (protege contra interceptação, mas exige plano de rotação).
- RLS por paciente no backend do portal como segunda barreira (CLAUDE.md G1).
- O paciente vê quem acessou seus dados, baixa os próprios dados e controla consentimentos (R11).

## 4. Portal web responsivo (MVP)

- Mesmo design system, com letra ajustável; funciona em celular sem app.
- Convite de uso único, curto e vinculado à conta; mensagens que não revelam se o CPF existe (SEC-AUT-6).
- Limite de tentativas antes do login (SEC-DIS-5).
- O portal só recebe conteúdo **publicado** pelo servidor local (ADR-0011); não consulta o banco da clínica.

## 5. Tablet de sala (1180×820)

- Exceção de tamanho do `docs/05`. Barra do paciente com dois identificadores sempre visível.
- Bipe da pulseira e triagem com bloqueio seguem as mesmas rotas do desktop.
- Sem dado de saúde guardado no aparelho.

## 6. Testes

- Integração G4 no backend do portal: paciente A não vê dado de B (rota e RLS), convite de uso único, link expirado.
- Ponta a ponta com a skill `e2e` (`@e2e-dev/mobile` para iOS e Android quando houver app; web para o portal), regras `E2E-*`.
- Acessibilidade com leitor de tela do sistema (VoiceOver, TalkBack) nas tarefas do paciente.

## 7. Checklist antes de liberar qualquer versão móvel

- [ ] ADR da tecnologia móvel aprovado e registrado em `docs/decisoes.md`
- [ ] Nenhuma regra de preço, permissão ou limite no app; tudo vem do contrato OpenAPI
- [ ] Nenhum segredo no pacote; varredura de segredos limpa
- [ ] Token só no armazenamento seguro do sistema; limpo no logout e na troca de conta
- [ ] Conteúdo oculto ao ir para segundo plano; nada de dado de saúde em cache persistente ou log
- [ ] Notificações sem conteúdo clínico
- [ ] Só laudo assinado e liberado; "conversar antes" respeitado
- [ ] Teste de acesso indevido (paciente A e B) e de RLS no backend do portal
- [ ] Acessibilidade com VoiceOver e TalkBack; letra ajustável
- [ ] Testes ponta a ponta com a skill `e2e` nas tarefas do paciente
- [ ] Atualização publicada com assinatura e proveniência (skill `devops-seguranca`)
- [ ] Revisão de prontidão assinada (`docs/modelos/prontidao-para-producao.md`)
