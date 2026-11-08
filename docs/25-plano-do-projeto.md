# 25 · Plano do projeto

> **Status:** proposta (v0.1, 08/11/2026), item **E8** do [docs/22](22-lacunas-e-roadmap.md).
> Equipe, prazo e orçamento **não foram informados pelo dono do produto**. Este documento é
> uma proposta profissional pela regra do [ADR-0019](adr/0019-decisoes-pelo-mais-profissional-com-lastro.md):
> toda premissa está na tabela do § 1 com ID e marcada "a confirmar". Mudou uma premissa,
> recalcula-se o que depende dela e registra-se em [decisoes.md](decisoes.md).
> Escopo funcional no [docs/04](04-escopo-funcional.md); roteiro no [docs/22](22-lacunas-e-roadmap.md);
> arquitetura no [docs/11](11-arquitetura-e-infraestrutura.md).

## 1. Premissas (a confirmar pelo dono do produto)

| ID | Premissa | Afeta | Situação |
|---|---|---|---|
| PREM-01 | O sistema é construído por uma equipe contratada (fornecedor) para a clínica; o contrato do § 7 é entre fornecedor e clínica | Contrato, orçamento | A confirmar |
| PREM-02 | Uma unidade, seis salas de exame, cerca de 90 atendimentos por dia (referência do mockup de contingência: 87 pacientes em 6 salas) | Infraestrutura, migração, carga | A confirmar |
| PREM-03 | 160 horas por mês por pessoa em dedicação integral | Orçamento | A confirmar |
| PREM-04 | Custo por hora de mercado no Brasil em 2026, contratação PJ, nas faixas da tabela do § 6.1 | Orçamento | A confirmar (valores de referência, **não verificados** com pesquisa salarial) |
| PREM-05 | Prazo até a virada total entre 15 e 20 meses a partir do início da Fase 0 | Cronograma, orçamento | A confirmar |
| PREM-06 | Servidores, nobreak, rede e periféricos novos (não reaproveitados); o inventário do docs/09 pode reduzir o valor | Orçamento de infraestrutura | A confirmar |
| PREM-07 | Os aparelhos aceitam worklist (MWL), MPPS e C-STORE; os que não aceitam passam pelo conversor | Escopo da Fase 2 | A confirmar no inventário de aparelhos |
| PREM-08 | Os fornecedores atuais entregam a exportação em até 60 dias após o pedido formal | Cronograma da migração | A confirmar |
| PREM-09 | Licenças da Medware e da Medilab mantidas até 16 semanas após o início da virada (docs/24 § 10); valor atual não informado | Orçamento | A confirmar |
| PREM-10 | PIX por um PSP dos citados no docs/02 (Mercado Pago, Efí ou Asaas), com tarifa por transação | Custo recorrente | A confirmar |
| PREM-11 | WhatsApp pela plataforma oficial da Meta, cobrada por mensagem de modelo conforme a tabela vigente | Custo recorrente | A confirmar |
| PREM-12 | NFS-e pelo padrão nacional, com o município da clínica aderente | Escopo do caixa | A confirmar |
| PREM-13 | Assinatura ICP-Brasil com certificado em nuvem por médico, carimbo do tempo de autoridade credenciada; cerca de 10 médicos | Custo recorrente | A confirmar |
| PREM-14 | Prazo de emissão da NFS-e após o atendimento definido pelo município; usado no modo contingência (docs/23 § 7.2) | Contingência | A confirmar |
| PREM-15 | Memed integrada sem custo para a clínica | Custo recorrente | A confirmar |
| PREM-16 | Portal do paciente e backup externo em nuvem no Brasil; volume de imagens de 10 a 30 TB nos primeiros 5 anos | Custo recorrente | A confirmar com o volume mensal de exames |
| PREM-17 | Faturamento TISS reescrito em Rust já no MVP (não roda temporariamente a partir da suíte Clinica, opção do ADR-0003) | Escopo e prazo da Fase 3 | A confirmar |
| PREM-18 | O visualizador OHIF pode ser usado para diagnóstico após avaliação regulatória (Anvisa); se não puder, compra-se visualizador diagnóstico registrado | Orçamento, Fase 2 | A confirmar |
| PREM-19 | Responsável clínico de segurança disponível 4 h por semana; um usuário-chave por setor 2 h por semana | Cronograma, aceite | A confirmar |
| PREM-20 | Propriedade do código: cessão total à clínica **ou** licença perpétua com código-fonte em depósito (escrow); proposta padrão: licença perpétua com escrow | Contrato | A confirmar |
| PREM-21 | Reserva de contingência de 15% sobre o custo de desenvolvimento | Orçamento | A confirmar |
| PREM-22 | Sustentação após a estabilização com 3 a 4 pessoas | Custo recorrente | A confirmar |
| PREM-23 | Horário de funcionamento: segunda a sexta, 07:00 às 19:00; sábado, 07:00 às 13:00 | SLA | A confirmar |
| PREM-24 | Meus Exames Bebê e telemedicina não são usados hoje; se forem, entram no MVP | Escopo do MVP | A confirmar |
| PREM-25 | Pelo menos o tech lead tem experiência comprovada em Rust em produção; os demais aprendem com esqueleto e revisão em par (docs/11 § 10) | Prazo | A confirmar |
| PREM-26 | Pagamento ao fornecedor por marcos atrelados ao aceite de cada fase | Contrato, fluxo de caixa | A confirmar |

## 2. Escopo do MVP

O MVP é o conjunto mínimo que permite **desligar a Medware e a Medilab** sem perder
função que a clínica usa hoje, mais os itens essenciais E1 a E8 do docs/22. O que não
impede o desligamento vai para depois.

| Módulo (docs/04) | MVP | Depois | Observação |
|---|---|---|---|
| Identidade, perfis, 2FA, auditoria | Sim | — | Base de tudo (docs/18) |
| Cadastro de paciente, Paciente 360º (sem mapa 3D) | Sim | Mapa anatômico realista | Mapa é diferencial, não substitui função atual |
| Agenda por sala, lista de espera, bloqueios | Sim | Agenda com vários recursos (I8) | — |
| Agendamento com guia e reserva provisória | Sim | — | Regra do docs/21 |
| Pedido de autorização pelo sistema (E4) | Sim | Webservice com mais operadoras | MVP: operadoras de maior volume por webservice; demais pelo portal com registro |
| Recepção, senhas, check-in, TV | Sim | — | — |
| Etiquetas e rastreio | Sim | — | docs/20 |
| Caixa do particular, PIX, NFS-e, orçamento | Sim | Maquininha integrada | docs/04 já põe a maquininha depois |
| Explorador de lastro e dossiê | Sim | — | docs/21 |
| Modo contingência (E6) | Sim | — | docs/23 |
| Sala de exame, worklist, MPPS, aparelhos, conversor | Sim | — | — |
| Triagem de segurança (E1) | Sim | Itens de ressonância | Ressonância depende de F4 |
| Laudo, modelos, frases, SR, assinatura ICP | Sim | Ditado com IA, rascunho de IA | IA exige avaliação regulatória (F2) e revisão sempre |
| Laudo estruturado e classificações (E2) | Sim, TI-RADS, BI-RADS e obstetrícia | Demais classificações | Prioridade pelos exames que a clínica faz |
| Exame externo (E3) | Sim | — | — |
| Visualizador | Sim (PREM-18) | MPR/3D avançado | — |
| Fluxo de laudos, entrega, impressão | Sim | Telerradiologia externa | — |
| Portal do paciente (convite, laudos, imagens, documentos) | Sim | App nativo | Portal substitui Meus Exames e MediWeb |
| Atendimento médico (consulta) | Sim, se a clínica faz consulta hoje | — | Teste ergométrico e MAPA usam evolução |
| Faturamento TISS e auditoria (E5) | Sim | — | — |
| Financeiro | Sim: contas a receber e a pagar, conciliação, repasse | DRE, centro de custo, previsão | Repasse é usado hoje (Medware) |
| Estoque | Sim: contraste e itens com lote e validade | Compras, curva ABC | Rastreio de lote é exigência de lastro |
| WhatsApp e NPS | Sim: confirmação, preparo, laudo pronto, opt-out | Robô de agendamento, campanhas, sugestão de resposta | — |
| Indicadores | Sim: produção, tempo de laudo, faltas, glosa | Painéis completos | — |
| Configurações | Sim | — | — |
| Migração (E7) | Sim | — | docs/24 |
| Estacionamento | — | Sim | Fase posterior (docs/11) |
| App do paciente, App do médico, Meus Exames Bebê | — | Sim (PREM-24) | — |
| Telemedicina | — | Sim (PREM-24) | CFM 2.314/2022 |
| Itens Importantes I1 a I9 | — | Sim, logo após o MVP | Ordem decidida no comitê |
| Itens Futuros F1 a F6 | — | Sim | — |

## 3. Fases, entregas e critérios de aceite

### 3.1 Fases

| Fase | Meses (estimativa) | Entregas | Marco de aceite |
|---|---|---|---|
| **0 · Descoberta e fundação** | 1 a 2 | Inventário de aparelhos, servidor e volumes; contrato assinado; ADRs 0002 a 0019 aprovados; frases de comportamento ([docs/27](27-frases-de-comportamento.md)) aprovadas; esqueleto do monorepo com a rota de ponta a ponta do docs/11 § 11; ambiente de homologação; teste de usabilidade com os mockups ([docs/26](26-teste-de-usabilidade.md)); pedido formal de exportação aos fornecedores | M0 |
| **1 · Atendimento administrativo** | 3 a 6 | Identidade, pacientes, agenda, agendamento com guia, autorização (E4), recepção, etiquetas, caixa com PIX e NFS-e, lastro, contingência (kit e lançamento), portal (convite) | M1 |
| **2 · Clínico e imagem** | 6 a 10 | Orthanc, worklist, MPPS, gateway DICOM, conversor, sala de exame, triagem (E1), laudo e laudo estruturado (E2), assinatura ICP, fluxo de laudos, visualizador, exame externo (E3), portal com laudos, WhatsApp básico | M2 |
| **3 · Faturamento e gestão** | 9 a 13 | Faturamento TISS, auditoria (E5), financeiro essencial, estoque com lote, indicadores básicos | M3 |
| **4 · Migração e virada** | 12 a 16 | Ondas do docs/24, piloto por sala, virada, estabilização | M4 (virada) e M5 (fim da janela de volta) |
| **5 · Depois do MVP** | 17 em diante | I1 a I9, apps, estacionamento, telemedicina, F1 a F6 | Por item |

As fases se sobrepõem de propósito: a Fase 3 começa quando o núcleo da Fase 2 está estável,
e a migração de imagens (Onda 1) começa durante a Fase 3.

### 3.2 Critérios de aceite comuns a todo módulo
Um módulo só é aceito quando:
1. Documento de design aprovado ([modelo](modelos/documento-de-design.md)).
2. Frases de comportamento aprovadas e todos os testes correspondentes verdes em `just test`, cobrindo caminho feliz, erros, repetição, limites e acesso indevido (G4).
3. Requisitos com ID ligados a teste na [matriz de rastreabilidade](modelos/rastreabilidade.md).
4. Riscos clínicos do módulo com controle implementado e testado; parecer do responsável clínico de segurança quando clínico ([pacote](modelos/pacote-responsavel-clinico.md)).
5. Teste de usabilidade das tarefas críticas aprovado: pelo menos 4 de 5 participantes sem erro e sem ajuda (docs/16 C4).
6. Latência p95 abaixo de 150 ms nas telas operacionais (docs/11 R2), medida em teste de carga.
7. Acessibilidade WCAG 2.2 AA verificada (axe e teclado).
8. Revisão de prontidão assinada antes de ir para produção ([modelo](modelos/prontidao-para-producao.md)).

### 3.3 Critérios específicos por módulo

| Módulo | Critério específico de aceite |
|---|---|
| Agenda e agendamento com guia | Duas reservas simultâneas do mesmo horário: uma só tem efeito; guia consumida de forma atômica e não reutilizável; reserva sem guia expira no prazo configurado |
| Autorização (E4) | Pedido enviado com chave de idempotência; resposta da operadora gravada como recebida e preenchendo a guia sem digitação, em pelo menos uma operadora real em homologação |
| Recepção e etiquetas | Check-in bloqueado sem dois identificadores; pulseira errada na sala bloqueia o exame; reimpressão invalida a anterior |
| Caixa | Total recalculado no servidor; desconto acima do limite só com autorização com reautenticação; fechamento esperado e contado; webhook do PIX duplicado não duplica recebimento |
| Contingência | Simulado completo em homologação com retomada em até 60 min e indicadores do docs/23 § 11 |
| Sala de exame e aparelhos | Cada aparelho do inventário recebe worklist e envia imagens com hash em homologação; estudo sem conciliação nunca vai ao laudo |
| Triagem (E1) | Exame com "precisa de aprovação médica" bloqueado até a conduta; aprovado pelo responsável clínico |
| Laudo e assinatura | Laudo assinado imutável; correção cria versão com motivo; assinatura ICP-Brasil válida em validador externo; só laudo assinado vai ao portal |
| Laudo estruturado (E2) | Categoria calculada pelo servidor com a versão da tabela gravada; casos de teste da tabela oficial conferidos por médico |
| Exame externo (E3) | Importado como somente leitura; nunca vira laudo da VidaCor; correção PIR guarda o original |
| Portal | Paciente A não vê dado de B (rota e RLS); link do convite de uso único |
| Faturamento e auditoria (E5) | Lote TISS validado no XSD da ANS; guia com erro bloqueante fora do lote; lote aceito por pelo menos uma operadora real |
| Financeiro | Conciliação bancária de um mês de homologação sem diferença; repasse com regra versionada |
| Estoque | Baixa de contraste não deixa saldo negativo com duas baixas simultâneas; recall por lote lista os pacientes |
| Migração | Critérios numéricos do docs/24 § 7.2 e critério de virada do § 8.2 |

## 4. Equipe recomendada

| Papel | Dedicação | Fases | Responsabilidade |
|---|---|---|---|
| Gerente de projeto | 0,5 | 0 a 4 | Cronograma, riscos, ritos, relatórios ao comitê |
| Arquiteto e tech lead (Rust) | 1,0 | 0 a 4 | Arquitetura, ADRs, revisão de código, dono técnico do núcleo |
| Desenvolvedor backend Rust sênior | 2,0 | 0 a 4 | Domínios do núcleo, banco, RLS |
| Desenvolvedor backend Rust pleno | 1,0 | 1 a 4 | Domínios administrativos, relatórios |
| Especialista em integração DICOM e saúde | 1,0 | 0 a 4 | Orthanc, worklist, MPPS, gateway, conversor, migração de imagens, IHE |
| Desenvolvedor frontend React/TypeScript sênior | 1,0 | 0 a 4 | Design system, telas críticas, Tauri |
| Desenvolvedor frontend pleno | 1,0 | 1 a 4 | Telas administrativas e portal |
| Designer de UX e UI | 0,5 | 0 a 4 | Telas a partir dos mockups, testes de usabilidade |
| Analista de qualidade e automação de testes | 1,0 | 0 a 4 | Testes de integração, carga, acessibilidade, E2E |
| DevOps e SRE | 0,5 | 0 a 4 | Infraestrutura local, CI/CD, observabilidade, backup |
| Desenvolvedor JVM (assinatura) | 0,25 | 2 | Serviço de assinatura ICP-Brasil (ADR-0008) |
| Analista de faturamento TISS | 0,5 | 1 a 4 | Regras por operadora, auditoria, conferência da migração |
| Analista de implantação e treinamento | 1,0 | 3 e 4 (cerca de 8 meses) | Treinamento por perfil, manuais, apoio na virada |
| Consultoria de segurança (AppSec) | Pontual | 2 e 4 | Modelagem de ameaças, teste de invasão externo antes da produção |
| Consultoria jurídica (LGPD e contratos) | Pontual | 0 e 4 | Contrato, contrato de operador, pedido aos fornecedores |
| Consultoria regulatória (Anvisa) | Pontual | 0 e 2 | Enquadramento do software e do visualizador |

**Do lado da clínica** (sem custo para o fornecedor, mas com tempo reservado): dono do produto,
responsável clínico de segurança (PREM-19), um usuário-chave por setor, TI da clínica,
encarregado de dados (DPO).

Total aproximado: **10,25 pessoas** em dedicação integral equivalente nas Fases 1 a 3,
mais implantação e consultorias.

## 5. Cronograma estimado

| Mês | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Fase 0 | x | x | | | | | | | | | | | | | | |
| Fase 1 | | | x | x | x | x | | | | | | | | | | |
| Fase 2 | | | | | | x | x | x | x | x | | | | | | |
| Fase 3 | | | | | | | | | x | x | x | x | x | | | |
| Migração (ondas 0 a 4) | | x | | | | | | | | x | x | x | x | x | x | |
| Piloto e virada | | | | | | | | | | | | | | | x | x |
| Usabilidade com a equipe | x | | | | x | | | | x | | | x | | | | |

- Estimativa de referência: **16 meses** até a virada total, faixa de **15 a 20 meses** (PREM-05).
- Janela de volta e estabilização: 2 meses após a virada.
- Datas no calendário só depois da confirmação das premissas; nenhuma virada em véspera de feriado ou fechamento de faturamento (docs/16 C5).

## 6. Orçamento estimado

Valores em reais, faixas de referência. **Não verificados** com cotação; servem para a
decisão de investimento e são substituídos por cotações reais na Fase 0.

### 6.1 Custo por hora de referência (PREM-04)

| Papel | R$/hora (faixa) | Dedicação | R$/mês (faixa) |
|---|---|---|---|
| Gerente de projeto | 120 a 170 | 0,5 | 9.600 a 13.600 |
| Arquiteto e tech lead | 180 a 250 | 1,0 | 28.800 a 40.000 |
| Backend Rust sênior | 140 a 200 | 2,0 | 44.800 a 64.000 |
| Backend Rust pleno | 100 a 140 | 1,0 | 16.000 a 22.400 |
| Integração DICOM | 160 a 230 | 1,0 | 25.600 a 36.800 |
| Frontend sênior | 120 a 170 | 1,0 | 19.200 a 27.200 |
| Frontend pleno | 90 a 130 | 1,0 | 14.400 a 20.800 |
| UX e UI | 100 a 150 | 0,5 | 8.000 a 12.000 |
| Qualidade e testes | 90 a 140 | 1,0 | 14.400 a 22.400 |
| DevOps e SRE | 140 a 200 | 0,5 | 11.200 a 16.000 |
| JVM (assinatura) | 130 a 180 | 0,25 | 5.200 a 7.200 |
| Analista TISS | 90 a 140 | 0,5 | 7.200 a 11.200 |
| **Total mensal da equipe** | | **10,25** | **204.400 a 293.600** |

### 6.2 Investimento até a virada

| Item | Faixa (R$) | Base |
|---|---|---|
| Equipe | 3,07 mi a 5,87 mi | Faixa baixa: 15 meses com taxa mínima; faixa alta: 20 meses com taxa máxima |
| Implantação e treinamento | 67 mil a 141 mil | 6 a 8 meses, R$ 70 a 110/h |
| Consultorias (AppSec e teste de invasão, jurídico, regulatório) | 85 mil a 210 mil | Pontuais |
| **Subtotal desenvolvimento** | **3,22 mi a 6,22 mi** | |
| Reserva de contingência (15%, PREM-21) | 0,48 mi a 0,93 mi | |
| Infraestrutura local (§ 6.3) | 185 mil a 346 mil | PREM-06 |
| **Total estimado** | **3,89 mi a 7,50 mi** | |

### 6.3 Infraestrutura local (compra única)

| Item | Faixa (R$) |
|---|---|
| Servidor principal (docs/11 § 7.1: 16 núcleos, 64 GB, NVMe em espelho, 4 discos em RAIDZ2) | 80 mil a 140 mil |
| Servidor de contingência com réplica | 35 mil a 60 mil |
| Nobreak senoidal online com autonomia de 30 min | 15 mil a 35 mil |
| Switch gerenciável com VLANs e firewall | 20 mil a 45 mil |
| 4 impressoras térmicas de etiqueta | 16 mil a 28 mil |
| 12 leitores de código de barras | 7 mil a 14 mil |
| 6 tablets de sala e de assinatura | 12 mil a 24 mil |

### 6.4 Custos recorrentes de operação (por mês, após a virada)

| Item | Faixa (R$/mês) | Premissa |
|---|---|---|
| Backup externo criptografado em nuvem no Brasil | 2 mil a 8 mil | PREM-16 |
| Hospedagem do portal em nuvem no Brasil | 1,5 mil a 4 mil | PREM-16 |
| PSP do PIX | Tarifa por transação, a cotar | PREM-10 |
| WhatsApp (Meta) | Por mensagem de modelo, conforme volume | PREM-11 |
| NFS-e | Sem custo no emissor nacional; intermediário, se usado, a cotar | PREM-12 |
| Certificados ICP-Brasil em nuvem e carimbo do tempo | Por médico e por carimbo, a cotar | PREM-13 |
| Memed | Sem custo | PREM-15 |
| Sustentação (3 a 4 pessoas) | 55 mil a 100 mil | PREM-22 |
| Licenças dos sistemas antigos durante a transição | Valor atual da clínica | PREM-09 |

### 6.5 Como reduzir, se necessário
- Rodar o faturamento TISS temporariamente a partir da suíte Clinica (ADR-0003), com contrato estável (PREM-17).
- Adiar financeiro além de contas e repasse, indicadores avançados e WhatsApp além do básico.
- Reaproveitar hardware que o inventário aprovar.
Cada redução é decisão do comitê e entra no registro de decisões.

## 7. Contrato com a clínica

### 7.1 Nível de serviço (SLA)

| Item | Compromisso |
|---|---|
| Disponibilidade no horário de funcionamento (PREM-23) | 99,9% ao mês, medida pelo monitoramento (docs/12) |
| RPO (perda máxima de dados) | 5 min |
| RTO (volta com o servidor de contingência) | 30 min |
| Primeira resposta S1 (atendimento parado, risco ao paciente) | 15 min, 24 horas por dia |
| Primeira resposta S2 (módulo indisponível) | 1 h no horário de funcionamento |
| Primeira resposta S3 (lentidão com contorno) | 1 dia útil |
| Primeira resposta S4 (incômodo, sugestão) | 5 dias úteis |
| Solução de S1 | Contorno em até 4 h; análise pós-incidente em até 5 dias úteis |
| Restauração de backup testada | Mensal, com relatório |
| Descumprimento | Desconto proporcional na mensalidade de sustentação, conforme tabela anexa |

### 7.2 Suporte e manutenção
- Canal único de chamados com número, gravidade, responsável e retorno do desfecho a quem abriu (docs/16 R14).
- Plantão para S1 fora do horário.
- Atualizações em janela de manutenção fora do expediente, com volta à versão anterior em um comando (docs/13).
- Correções de segurança aplicadas no prazo da gravidade; incidentes comunicados no formato do docs/16 R13.

### 7.3 Propriedade e devolução dos dados
- **Os dados são da clínica**, sempre: cadastros, prontuário, imagens, laudos, financeiro, logs e lastro.
- Devolução a qualquer momento e no fim do contrato, sem custo adicional, em até 30 dias do pedido, em formatos abertos e documentados: dump do PostgreSQL e CSV por tabela com dicionário de dados; imagens em DICOM; laudos e documentos em PDF/A com assinatura; lastro com hash conferível.
- Após a devolução confirmada por escrito, o fornecedor elimina as cópias que mantiver e entrega termo de eliminação, ressalvado o que a lei obriga a guardar.
- Transição assistida de até 90 dias para outro fornecedor, se a clínica pedir.

### 7.4 Propriedade do código (PREM-20)
- Proposta padrão: **licença perpétua, irrevogável e com direito de modificação**, com código-fonte, scripts de instalação e documentação em **depósito de código (escrow)**, liberados à clínica se o fornecedor encerrar as atividades ou descumprir a sustentação.
- Alternativa: cessão total dos direitos patrimoniais à clínica, com preço ajustado.
- Componentes de terceiros seguem as próprias licenças (THIRD_PARTY_NOTICES.md).

### 7.5 LGPD: fornecedor como operador
Contrato de operador anexo ao contrato principal, com:
- tratamento só conforme instruções documentadas da clínica (controladora);
- confidencialidade da equipe e acesso mínimo (TI sem dado clínico, docs/18);
- lista de suboperadores (nuvem do portal, backup, PSP, WhatsApp, assinatura), com aviso prévio de troca e direito de objeção;
- dados mantidos no Brasil; transferência internacional só com autorização escrita e base legal;
- comunicação de incidente à clínica em até **24 horas** da ciência, para que ela cumpra o prazo de comunicação à ANPD e aos titulares;
- apoio aos pedidos de titulares e ao relatório de impacto (RIPD);
- auditoria pela clínica ou terceiro indicado, uma vez por ano ou após incidente;
- devolução e eliminação conforme § 7.3.

### 7.6 Garantia
- Correção sem custo de defeitos encontrados em até **12 meses** após o aceite de cada módulo.
- Defeito é comportamento diferente das frases de comportamento aprovadas, dos requisitos com ID ou deste contrato.
- Mudança de comportamento pedida depois do aceite é mudança de escopo (§ 9.3), não defeito.

### 7.7 Outras cláusulas recomendadas
Pagamento por marcos com aceite (PREM-26); mudança de escopo por pedido formal com
impacto em prazo e custo; seguro de responsabilidade civil profissional e de riscos
cibernéticos do fornecedor; confidencialidade; foro e mediação.

## 8. Riscos do projeto

| Risco | Probabilidade | Impacto | Mitigação | Dono |
|---|---|---|---|---|
| Premissas de orçamento e prazo não confirmadas | Alta | Alto | Tabela de premissas revisada na Fase 0; recálculo registrado | Gerente de projeto |
| Equipe sem experiência em Rust | Média | Alto | PREM-25; esqueleto, convenções, revisão em par (docs/11 § 10) | Tech lead |
| Aparelhos com DICOM limitado | Média | Alto | Inventário na Fase 0; conversor; DICOM Conformance Statement de cada aparelho | Especialista DICOM |
| Fornecedores atuais não entregam dados | Média | Alto | Pedido formal na Fase 0; base contratual e LGPD (docs/24 § 3) | Dono do produto |
| Assinatura ICP-Brasil e carimbo do tempo | Média | Médio | Serviço JVM isolado; homologação com certificado real na Fase 2 | Tech lead |
| Enquadramento regulatório do visualizador | Média | Alto | Consultoria regulatória na Fase 0; alternativa registrada (PREM-18) | Dono do produto |
| Disponibilidade da equipe da clínica para aprovações e testes | Alta | Médio | Agenda fixa de ritos (§ 9); PREM-19 | Gerente de projeto |
| Escopo cresce durante o projeto | Alta | Alto | Mudança de escopo formal; itens novos vão para depois do MVP salvo decisão do comitê | Comitê |
| Validação XSD do TISS e regras por operadora | Média | Médio | Analista TISS desde a Fase 1; massa de testes da suíte Clinica | Analista TISS |
| Virada com problema grave | Baixa | Alto | Piloto por sala, critério de virada, plano de volta (docs/24 § 8) | Comitê de virada |
| Erro clínico causado pelo sistema | Baixa | Muito alto | Registro de riscos clínicos, parecer do responsável clínico, usabilidade, testes G4 | Responsável clínico |

## 9. Governança

### 9.1 Ritos

| Rito | Frequência | Participantes | Saída |
|---|---|---|---|
| Reunião diária da equipe | Diária, 15 min | Equipe do fornecedor | Impedimentos |
| Revisão da iteração com a clínica | Quinzenal | Equipe, dono do produto, usuários-chave | Aceite ou ajuste do que foi mostrado em homologação |
| Planejamento da iteração | Quinzenal | Equipe, dono do produto | Itens priorizados |
| Comitê do projeto | Mensal | Direção da clínica, dono do produto, gerente de projeto, tech lead | Situação de prazo, custo, riscos; decisões de escopo |
| Revisão de risco clínico | Mensal e antes de cada módulo clínico | Responsável clínico de segurança, tech lead, designer | Registro de riscos atualizado; pareceres |
| Revisão de mudança (antes de lançar) | A cada lançamento | Dono técnico, dono do produto, responsável clínico se clínico | Prontidão assinada |
| Retrospectiva | Mensal | Equipe | Ações de melhoria |

### 9.2 Quem assina o quê

| Artefato ou decisão | Assina |
|---|---|
| Premissas deste documento | Dono do produto |
| Frases de comportamento (docs/27) | Dono do produto |
| Mudança em fluxo clínico; registro de riscos clínicos; novo alerta bloqueante | Responsável clínico de segurança |
| ADR | Tech lead e dono do produto |
| Migration de banco | Revisor humano (G6) |
| Aceite de módulo (§ 3) | Dono do produto e usuário-chave do setor |
| Revisão de prontidão | Dono do produto, dono técnico e responsável clínico (docs/16 C5) |
| Lotes de migração | Quem conferiu e responsável da clínica pelo dado (docs/24 § 7) |
| Decisão de virada e de volta atrás | Comitê de virada (docs/24 § 8) |
| Mudança de escopo, prazo ou orçamento | Comitê do projeto, com aprovação da direção |
| Contrato e contrato de operador | Direção da clínica e representante do fornecedor |

### 9.3 Mudança de escopo
Pedido escrito com motivo, impacto em prazo, custo e risco, decisão do comitê e linha no
registro de decisões. Nada entra no MVP por conversa de corredor.

### 9.4 Relatórios
Relatório mensal ao comitê: entregas, marcos, custo realizado contra previsto, riscos,
indicadores de qualidade (testes, defeitos, latência) e decisões tomadas.
