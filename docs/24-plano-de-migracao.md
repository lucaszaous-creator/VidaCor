# 24 · Plano de migração (Medware, Mediworks, Medilab e conversor DICOM)

> **Status:** proposta (v0.1, 08/11/2026), item **E7** do [docs/22](22-lacunas-e-roadmap.md).
> Sistemas de origem descritos no [docs/03](03-levantamento-sistemas-atuais.md). Liga-se ao
> lastro ([docs/21](21-lastro-e-rastreabilidade.md)), às garantias G2, G3, G4 e G6
> ([docs/15](15-padrao-de-qualidade.md)), à proteção do usuário ([docs/16](16-protecao-do-usuario.md) C5)
> e ao modo contingência ([docs/23](23-modo-contingencia.md)), usado também no plano de volta.
> Volumes e formatos marcados "a confirmar" dependem do inventário com a clínica e das
> exportações dos fornecedores; até lá estão **não verificados**.

## 1. Princípios

1. **Nada se perde, nada se inventa.** Cada registro de origem termina carregado, rejeitado com motivo ou declarado fora do escopo. Contagem de entrada igual à soma das três.
2. **Origem sempre visível.** Todo registro migrado leva lastro de origem (§ 9) e aparece marcado como migrado.
3. **Laudo antigo continua sendo do sistema antigo.** Entra como documento histórico somente leitura e nunca vira laudo da VidaCor.
4. **Paciente não é unido sem segunda pessoa.** Duplicados são reportados e resolvidos por duas pessoas, um a um.
5. **Carga idempotente.** Rodar o mesmo lote de novo não duplica (índice único por sistema, entidade e id de origem).
6. **Dado real só em infraestrutura de produção.** Ensaios com dado real rodam em instância isolada no servidor da clínica, com os mesmos controles de produção (G4: nunca dado real de paciente fora de produção). Desenvolvimento e testes automáticos usam extratos sintéticos ou anonimizados.
7. **Gradual e reversível.** Virada por sala, com período em paralelo e plano de volta testado.

## 2. Inventário do que existe

Quantidades a preencher no levantamento (Onda 0). "Migra" indica o destino no VidaCor.

### 2.1 Medware (gestão, laudos de eco e USG, comDICOM)

| Dado | Uso hoje | Migra? | Destino no VidaCor | Observação |
|---|---|---|---|---|
| Pacientes | Cadastro, contatos, convênio | Sim | `pacientes` | Base da deduplicação (§ 5) |
| Médicos (corpo clínico e solicitantes) | Agenda, laudos, repasse | Sim | `Corpo-Clinico` | Corpo clínico só vale após segunda conferência (docs/04) |
| Convênios, planos, tabelas | Agenda, faturamento | Sim, como rascunho | `Convenios-Tabelas` | Tabela entra pendente e só vale após conferência de segunda pessoa |
| Exames e procedimentos (códigos internos) | Agenda, preço | Sim, por tabela de-para | `Exames-Procedimentos` | De-para código interno para TUSS revisado pelo faturamento |
| Agenda futura | Operação | Sim | `agenda` | Com guias e senhas |
| Agenda passada | Histórico | Sim, como histórico | `agenda` (somente leitura) | Sem gerar etiquetas nem cobranças |
| Autorizações e guias | Convênio | Sim, as válidas e as pendentes | `Agendamento-Guia`, `Autorizacao` | Consumo de guia preservado (saldo) |
| Faturamento TISS (lotes, glosas, recursos) | Faturamento | Só os **abertos** | `faturamento` | Lotes fechados ficam no arquivo histórico (§ 2.5) |
| Financeiro (contas a receber e a pagar) | Financeiro | Só os **saldos abertos** e o plano de contas | `financeiro` | Movimento histórico fica no arquivo |
| Repasse médico pendente | Financeiro | Sim, os pendentes | `financeiro` | Regra de repasse recadastrada com versão |
| Estoque | Estoque | **Não** migra saldo | `estoque` | Saldo por inventário físico na virada, com lote e validade |
| Laudos Flex (eco, USG) | Laudos | Sim | Documento histórico (§ 6) | Pedir PDF assinado e metadados |
| Modelos de laudo e banco de frases | Laudos | Sim, como rascunho | `Modelos-Laudo` | Só valem após aprovação da responsável técnica |
| Imagens comDICOM | PACS de eco e USG | Sim | Orthanc | Por DICOM (§ 4.1) |
| Meus Exames (acessos do portal) | Portal | **Não** migra credencial | Portal novo | Paciente recebe convite novo (link de uso único) |
| Paciente Conectado (consentimentos, opt-out WhatsApp) | Comunicação | Sim | `comunicacao`, consentimentos | Opt-out sempre migra; consentimento sem texto versionado entra como "legado, a recolher" |
| Usuários e senhas da equipe | Acesso | **Não** | Keycloak | Contas novas, individuais, com 2FA |
| Logs e auditoria | Auditoria | Não carrega; arquiva | Arquivo histórico | Exportação guardada com hash |

### 2.2 Medilab e Mediworks (RIS/PACS, laudos de RX e TC)

| Dado | Uso hoje | Migra? | Destino | Observação |
|---|---|---|---|---|
| Pacientes do RIS | Agenda de exames | Sim | `pacientes` | Cruzados com os da Medware na deduplicação |
| Agenda do RIS (MediClinic/Mediflow) | Operação | Futura sim; passada como histórico | `agenda` | Confirmar se a agenda principal é a da Medware |
| Laudos (Mediworks) | Laudos | Sim | Documento histórico (§ 6) | PDF assinado e metadados |
| Modelos de laudo | Laudos | Sim, como rascunho | `Modelos-Laudo` | Aprovação da responsável técnica |
| Imagens (MediSystem PACS) | RX, TC, demais | Sim | Orthanc | Maior volume da migração |
| Entregas no MediWeb | Portal | Registro de entregas como histórico | Lastro de entrega | Acessos não migram |
| Elegibilidade e autonotificação | Recepção | Não (configuração refeita) | `integracoes` | — |

### 2.3 Conversor DICOM (MediPrint ou equivalente)

| Item | Migra? | Observação |
|---|---|---|
| Objetos já convertidos (PDF do ergométrico e do MAPA em DICOM) | Sim, junto com o PACS de origem | Já estão como DICOM |
| Configuração (AE Title, modalidades, regras) | Não; serve de especificação | O conversor do VidaCor (`Aparelhos`) é configurado do zero |
| Arquivos ainda não convertidos | Levantar | Se houver, convertidos com o conversor novo e marcados como migrados |

### 2.4 Fora do escopo da migração
CD/DVD (retirado, docs/22), credenciais de usuários e pacientes, filas de mensagens
não enviadas, relatórios gerenciais prontos (são recalculados pelo VidaCor).

### 2.5 Arquivo histórico
O que não é carregado como registro vivo (movimento financeiro fechado, lotes TISS
fechados, logs, relatórios) é guardado como **arquivo histórico**: exportação original,
hash SHA-256, índice pesquisável por paciente, competência e número, acesso pelo perfil
correspondente (docs/18) e retenção pelo prazo legal de cada tipo de dado (docs/21 § 5).

## 3. Como obter os dados dos fornecedores

1. **Pedido formal** da clínica a cada fornecedor, por escrito, com base:
   - no contrato vigente (cláusulas de devolução de dados e fim de contrato);
   - na LGPD: a clínica é **controladora** e os fornecedores são **operadores**, que tratam dados segundo as instruções do controlador (art. 39); o pedido também atende ao direito de portabilidade dos titulares (art. 18, V).
2. O pedido especifica:
   - formato aberto e documentado: dump do banco ou CSV por tabela, com **dicionário de dados**;
   - laudos em PDF (preferencialmente PDF/A) com a assinatura digital original, mais metadados (paciente, data, exame, médico, accession, versão, situação);
   - imagens pelo PACS (DICOM), com o PACS de origem liberado para consulta e recuperação (C-FIND e C-MOVE) a partir do Orthanc;
   - prazo, meio seguro de entrega (mídia cifrada ou transferência cifrada, nunca e-mail), hash de cada arquivo;
   - uma extração completa para ensaio e uma extração **delta** na virada;
   - manutenção do acesso de leitura aos sistemas antigos por 90 dias após a virada.
3. Se o fornecedor atrasar ou negar: notificação formal, escalonamento pela direção e apoio da consultoria jurídica prevista no [docs/25](25-plano-do-projeto.md) § 4 (prazo de entrega em PREM-08). Último recurso: exportação pelos relatórios do próprio sistema, documentada e conferida com o mesmo rigor. Não se usa captura de tela nem acesso ao banco do fornecedor sem autorização escrita.
4. Todo arquivo recebido é registrado (fornecedor, data, quem recebeu, hash) antes de qualquer processamento.

## 4. Estratégia por tipo de dado

### 4.1 Imagens (PACS por DICOM)
- **Caminho principal:** o Orthanc consulta o PACS de origem (C-FIND por estudo) e pede a transferência (**C-MOVE**), recebendo por **C-STORE**. Se o PACS de origem não aceitar C-MOVE, o fornecedor envia os arquivos DICOM e eles entram por C-STORE (`storescu`) a partir de mídia cifrada.
- **Ordem:** estudos dos últimos 2 anos primeiro (são os anteriores mais usados em comparação), depois os mais antigos.
- **Ritmo:** fora do horário de pico, com limite de banda, para não afetar a operação atual.
- **Verificação por contagem:** para cada estudo, número de séries e de instâncias na origem (C-FIND) igual ao recebido; lista de SOP Instance UID igual.
- **Verificação por hash:** SHA-256 de cada objeto na chegada, gravado no lastro. Em amostra (§ 7), o objeto é exportado da origem e comparado: hash do arquivo quando a sintaxe de transferência é a mesma; hash dos dados de pixel decodificados quando a origem converte a sintaxe.
- **Objetos originais não são alterados na carga.** A ligação estudo e paciente VidaCor é feita por tabela de conciliação. Correção de identificação nas imagens só pelo perfil IHE PIR, com original guardado.
- Estudos migrados entram como **estudos históricos já laudados**: servem de anterior para comparação e **nunca** entram na fila de laudo.
- Estudo sem paciente conciliado fica na fila de conciliação de migração; nunca vai ao laudo nem ao portal.

### 4.2 Cadastros, agenda, guias e financeiro aberto (exportação dos fornecedores)
Fluxo em três áreas separadas no PostgreSQL:

| Área | O que tem | Quem acessa |
|---|---|---|
| `migracao_bruto` | Arquivos recebidos, sem alteração, com hash | Equipe de migração, só leitura |
| `migracao_preparo` | Dados transformados pelo mapeamento, com resultado de validação e candidatos a duplicado | Equipe de migração e revisores da clínica |
| Tabelas do VidaCor | Registros carregados | Pelos perfis normais (docs/18) |

- A carga passa pelas **mesmas validações do domínio** (CPF, datas, TUSS, regra da guia), não por inserção direta que pule invariantes.
- Papel de banco próprio para a carga (`migracao_carga`), só com `INSERT` nas tabelas de destino, sem `BYPASSRLS`, auditado e **desativado após a virada**.
- O mapeamento (de-para de campos e códigos) fica versionado no repositório; cada lote grava a versão usada.
- Registro que falha validação vai para o relatório de rejeitados com motivo, nunca é corrigido "no chute". Correção de dado é feita por pessoa da clínica, com registro.

### 4.3 Laudos antigos
Ver § 6.

## 5. Deduplicação de pacientes

O mesmo paciente costuma existir na Medware e na Medilab, e às vezes duas vezes no mesmo sistema.

1. **Chave de comparação:** dois identificadores (nome completo normalizado e data de nascimento), mais CPF, nome da mãe e telefone como apoio.
2. **Classificação dos candidatos:**

| Faixa | Critério | Tratamento |
|---|---|---|
| Forte | CPF válido igual, nascimento igual, nome normalizado igual | Revisão individual por duas pessoas |
| Provável | Nascimento igual e nome semelhante (similaridade de trigramas acima do limiar), ou CPF igual com nome diferente | Revisão individual por duas pessoas, com documento quando necessário |
| Fraco | Nome igual e nascimento diferente, ou só telefone igual | Relatório; não vira candidato a união sem nova evidência |

3. **Nunca se une sozinho**, nem na faixa forte. A união exige uma pessoa que propõe e uma segunda que aprova, um paciente por vez, com os dois identificadores na tela (mockup `Pacientes-Lista`). Nada clínico em lote (docs/19).
4. A revisão acontece no ambiente de ensaio, nas semanas antes da virada, priorizando pacientes com agendamento futuro e atendimento nos últimos 24 meses.
5. Candidatos não revisados até a virada entram como cadastros separados, ligados como "possível duplicado", e seguem na fila de duplicados. A recepção vê o alerta ao buscar o paciente.
6. A união guarda os dois cadastros de origem, quem propôs, quem aprovou e o motivo (docs/21 § 3.1). Desfazer é possível pela mesma via, com registro.
7. Cadastro com CPF inválido entra sem CPF e com pendência. Cadastro sem data de nascimento entra marcado "identificação incompleta" e bloqueia ato clínico até ser completado no check-in.

## 6. Laudos antigos como documentos históricos

| Regra | Como |
|---|---|
| Tipo de registro | `documento_historico` (não é `laudo`) |
| Origem marcada | Sistema, id de origem, médico que assinou na origem, data original, lote de migração |
| Conteúdo | PDF recebido, com hash SHA-256; se o fornecedor entregar só texto ou RTF, o original é guardado e um PDF de leitura é gerado com a faixa "Documento migrado de <sistema>; original guardado" |
| Assinatura | A assinatura digital original é validada na carga; o resultado (válida, inválida, sem assinatura) fica no lastro e aparece na tela |
| Somente leitura | Não se edita, não se retifica, não se reassina no VidaCor |
| Nunca vira laudo da VidaCor | Não entra na fila de laudo, não é publicado como laudo novo, não gera faturamento |
| Uso | Aparece na linha do tempo do paciente com a marca "Laudo histórico (Medware)" ou "(Medilab)"; pode ser aberto como anterior ao laudar |
| Portal | Só aparece ao paciente se já estava publicado no portal antigo, e com a mesma marca |
| Correção | Se um laudo antigo estiver errado, o médico registra uma **nota** ligada a ele, em documento separado; o histórico não muda |

## 7. Carga de teste e conferência por amostragem

### 7.1 Ensaios
- **Ensaio 1** (Onda 2): cadastros, médicos, convênios. Mede qualidade do dado e volume de duplicados.
- **Ensaio 2** (Onda 3): soma laudos históricos e modelos.
- **Ensaio geral** (Onda 4): tudo, com a extração completa, cronometrado como se fosse a virada.
- Cada ensaio roda em instância isolada no servidor da clínica, com controles de produção, e é apagado ao fim com registro de quem apagou e quando (a instância de ensaio não é registro clínico).

### 7.2 Critérios numéricos de aceite por lote

| Entidade | Conferência automática (100%) | Amostra manual | Critério de aceite |
|---|---|---|---|
| Pacientes | Contagem origem = carregados + rejeitados + fora do escopo | 400 aleatórios (ou todos, se menos), comparados com a tela do sistema antigo | 0 erro em nome, nascimento, sexo e CPF; até 1% em contato e endereço |
| Médicos e convênios | Contagem e CRM/UF e registro ANS válidos | 100% conferidos por segunda pessoa | 0 erro |
| Agenda futura | Contagem por dia e sala igual ao relatório da origem | 100 aleatórios | 0 erro em paciente, data, hora, sala e exame |
| Guias abertas | Contagem e saldo por guia | 100% conferidas pelo faturamento | 0 erro em número, senha, validade e saldo |
| Financeiro aberto | Soma por conta igual ao centavo | 50 títulos aleatórios | 0 diferença de valor |
| Laudos históricos | Contagem e hash de 100% dos arquivos | 300 aleatórios abertos e comparados com a origem | 0 erro de paciente, data ou exame; 100% legíveis |
| Imagens | Contagem de séries e instâncias de 100% dos estudos | 1% dos estudos (mínimo 200) por hash; 20 estudos abertos por médico no visualizador | 0 divergência |

- A amostra de 400 dá confiança de 95% com margem de 5% para a taxa de erro.
- Lote que falha é rejeitado inteiro, corrigido no mapeamento ou na origem e recarregado (idempotente); a nova conferência usa **nova amostra**.
- Cada conferência gera relatório assinado por quem conferiu, com a amostra sorteada e a semente do sorteio.

## 8. Período em paralelo, virada e volta atrás

### 8.1 Período em paralelo (piloto por sala)
- A virada não é da clínica inteira de uma vez (docs/16 C5). Começa por **uma sala** de ultrassom ou eco por 2 semanas, depois grupos de salas por semana.
- Durante o paralelo, a agenda da sala piloto é gerida só no VidaCor; as demais seguem nos sistemas antigos. A recepção sabe por sala qual sistema usar (sinalização na tela e na sala).
- **Carga delta diária** do que mudou nos sistemas antigos (pacientes novos, agenda), idempotente pelo id de origem.
- **Comparação automática diária**: atendimentos, recebimentos e guias da sala piloto no VidaCor contra o esperado da agenda migrada; diferenças viram itens com dono.
- Nenhum ato é digitado nos dois sistemas ao mesmo tempo (evita divergência por dupla digitação).

### 8.2 Critério de virada (passa ou não passa)
A virada total só acontece com **todos** os itens:

1. Todos os lotes aceitos (§ 7.2) no ensaio geral, dentro da janela prevista.
2. Nenhum incidente S1 ou S2 aberto no piloto.
3. Teste de usabilidade aprovado ([docs/26](26-teste-de-usabilidade.md)) e revisão de prontidão assinada ([modelo](modelos/prontidao-para-producao.md)).
4. Pelo menos 95% dos usuários treinados por perfil.
5. Backup e restauração testados no servidor de produção; simulado de contingência feito.
6. Extração delta final recebida e conferida.
7. Acesso de leitura aos sistemas antigos garantido por contrato por 90 dias.
8. Data fora de véspera de feriado e de fechamento do faturamento.

Decisão registrada pelo comitê de virada: dono do produto, responsável clínico de segurança e dono técnico.

### 8.3 Plano de volta atrás
- **Janela:** 30 dias após a virada total. Licenças dos sistemas antigos mantidas até o fim da janela.
- **Gatilhos:** S1 sem solução em 4 horas com impacto no atendimento; perda ou corrupção de dado confirmada; erro de identificação de paciente causado pela migração; impossibilidade de faturar dentro do prazo das operadoras.
- **Decisão:** comitê de virada, registrada com motivo.
- **Como:**
  1. Declara-se contingência (docs/23) enquanto se volta.
  2. Os sistemas antigos voltam a ser o registro oficial.
  3. Tudo o que foi criado no VidaCor desde a virada é listado em relatório por paciente e lançado nos sistemas antigos pelo mesmo procedimento das fichas de contingência.
  4. Durante a janela, o Orthanc **reencaminha cópia** de cada estudo novo ao PACS antigo, para que a volta não perca imagens.
  5. O VidaCor fica somente leitura; nada é apagado.
- O plano de volta é ensaiado uma vez antes da virada, em homologação.

## 9. Lastro de cada registro migrado

Além dos metadados obrigatórios do docs/21 § 1:

| Campo | Conteúdo |
|---|---|
| `origem` | `migracao` |
| `sistema_origem` | `medware`, `medilab`, `mediworks` ou `conversor` |
| `entidade_origem` | Tabela ou tipo no sistema de origem |
| `id_origem` | Identificador no sistema de origem, como recebido |
| `hash_origem` | SHA-256 do registro de origem serializado de forma canônica (ou do arquivo, para laudos e imagens) |
| `arquivo_origem` | Nome e hash do arquivo de exportação de onde veio |
| `lote_migracao` | Código do lote (ex.: `MIG-2027-012`), com data da extração e da carga |
| `versao_mapeamento` | Versão do mapeamento aplicada |
| `carregado_por` | Papel de carga e pessoa responsável pelo lote |
| `conferencia` | Relatório de conferência que aceitou o lote |

Regras:
- Índice único (`sistema_origem`, `entidade_origem`, `id_origem`) em cada destino (G2, G3).
- Qualquer código de origem leva ao registro VidaCor, e vice-versa, pelo explorador de lastro.
- A união de duplicados mantém os dois `id_origem`.

## 10. Cronograma em ondas

Semanas relativas ao início da virada (V), quando entra a extração delta final. A virada total é a liberação do último grupo de salas (até V+3). As datas reais dependem do plano do projeto
([docs/25](25-plano-do-projeto.md)).

| Onda | Quando | O que | Saída |
|---|---|---|---|
| 0 · Preparação | V-24 a V-16 | Pedido formal aos fornecedores, inventário com volumes, dicionários de dados, mapeamento inicial | Inventário assinado; exportações de ensaio recebidas |
| 1 · Imagens | V-16 a V | Transferência DICOM em segundo plano, mais antigos por último; delta contínuo | 100% dos estudos conferidos por contagem |
| 2 · Cadastros | V-12 a V-2 | Ensaio 1; revisão de duplicados por duas pessoas | Lote de pacientes aceito; fila de duplicados priorizada |
| 3 · Laudos e modelos | V-10 a V-4 | Ensaio 2; validação de assinaturas; aprovação dos modelos pela responsável técnica | Laudos históricos aceitos; modelos aprovados |
| 4 · Operação | V-4 a V-1 | Ensaio geral cronometrado; agenda futura, guias e financeiro aberto; inventário físico do estoque; ensaio do plano de volta | Decisão de virada |
| Piloto | V-2 a V | Sala piloto no VidaCor com delta diário | Comparação diária sem pendência crítica |
| Virada | V | Extração delta final, carga, conferência, liberação por grupos de salas até V+3 | Clínica inteira no VidaCor |
| Estabilização | V a V+8 | Janela de volta aberta (30 dias após a virada total), suporte reforçado | Encerramento da janela de volta |
| Arquivamento | V+16 | Fim do acesso de leitura aos sistemas antigos; arquivo histórico conferido | Termo de encerramento da migração |

## 11. Riscos e mitigação

| Risco | Efeito | Mitigação |
|---|---|---|
| Fornecedor atrasa ou nega a exportação | Atraso da virada | Pedido formal na Onda 0, base contratual e LGPD, escalonamento jurídico, relatórios do próprio sistema como último recurso |
| Formato proprietário sem dicionário | Mapeamento errado | Exigir dicionário no pedido; validar com amostras na tela do sistema antigo |
| PACS de origem sem C-MOVE | Imagens não saem por rede | Exportação de arquivos DICOM em mídia cifrada |
| Volume de imagens maior que o estimado | Transferência não termina antes da virada | Começar na Onda 1; priorizar 2 anos; virada não depende dos estudos antigos, que seguem em segundo plano |
| Duplicados em grande número | Revisão não termina | Priorização por agenda futura; restante entra separado com alerta e segue na fila |
| Dado de identificação ruim (CPF inválido, nascimento ausente) | Erro de identificação | Rejeição ou pendência; bloqueio de ato clínico até completar |
| Laudo antigo sem vínculo com paciente | Documento órfão | Fila de conciliação; não aparece em paciente nenhum até conciliar com dois identificadores |
| Assinatura digital original inválida ou ausente | Valor jurídico questionável | Resultado da validação registrado e exibido; original guardado como recebido |
| Vazamento em ambiente de ensaio | Incidente LGPD | Ensaio só no servidor da clínica, com controles de produção; instância apagada com registro |
| Erro por dupla digitação no paralelo | Dado divergente | Cada sala em um sistema só; nenhuma dupla digitação |
| Janela de virada estoura | Clínica sem sistema na segunda-feira | Ensaio geral cronometrado; contingência pronta; volta aos sistemas antigos |
| Custo de licenças durante o paralelo | Orçamento | Previsto no plano do projeto (docs/25) |

## 12. Requisitos

| ID | Requisito | Critério de aceite |
|---|---|---|
| MIG-01 | Inventário por sistema com volumes, formatos e destino de cada dado | Inventário assinado pela clínica na Onda 0 |
| MIG-02 | Pedido formal de exportação a cada fornecedor com base contratual e LGPD | Protocolo do pedido arquivado |
| MIG-03 | Todo arquivo recebido registrado com hash antes do processamento | Registro de recebimento no lastro |
| MIG-04 | Contagem de origem igual a carregados + rejeitados + fora do escopo, por entidade | Relatório de reconciliação com diferença 0 |
| MIG-05 | Carga idempotente por (`sistema_origem`, `entidade_origem`, `id_origem`) | Teste: recarregar o lote não cria registro novo |
| MIG-06 | Carga pelas validações do domínio, com papel próprio só de `INSERT`, sem `BYPASSRLS`, desativado após a virada | Teste de permissão do papel; registro da desativação |
| MIG-07 | Imagens por C-MOVE/C-STORE, verificadas por contagem de séries e instâncias em 100% dos estudos | Relatório de contagem |
| MIG-08 | SHA-256 de cada objeto na chegada e comparação em amostra de 1% (mínimo 200 estudos) | 0 divergência |
| MIG-09 | Objetos DICOM originais não alterados na carga; correção só por IHE PIR com original guardado | Teste de imutabilidade do objeto |
| MIG-10 | Estudo migrado nunca entra na fila de laudo; sem conciliação não aparece em paciente | Teste de fila |
| MIG-11 | Deduplicação por dois identificadores; união só com duas pessoas, um paciente por vez | Teste: união sem segunda pessoa recusada (403) |
| MIG-12 | Candidato não revisado entra separado e ligado como "possível duplicado" | Alerta na busca do paciente |
| MIG-13 | Laudo antigo carregado como `documento_historico`, somente leitura, com origem marcada | Teste: edição, assinatura e publicação como laudo recusadas |
| MIG-14 | Assinatura digital original validada e resultado registrado | Resultado visível na tela e no lastro |
| MIG-15 | Modelos de laudo e tabelas de convênio entram como rascunho até aprovação | Teste: rascunho não usável em laudo ou preço |
| MIG-16 | Credenciais de usuários e pacientes não migram; pacientes recebem convite novo | Nenhum campo de senha no mapeamento |
| MIG-17 | Opt-out de comunicação sempre migra; consentimento sem texto versionado entra como "legado, a recolher" | Teste de envio bloqueado para opt-out migrado |
| MIG-18 | Lastro de migração com os campos do § 9 em todo registro migrado | Teste de schema |
| MIG-19 | Conferência por amostragem com os critérios do § 7.2, relatório assinado e semente do sorteio | Relatórios por lote |
| MIG-20 | Ensaios com dado real só em instância isolada no servidor da clínica, apagada com registro | Registro de criação e remoção |
| MIG-21 | Virada por sala, com carga delta diária e comparação automática | Relatório diário do piloto |
| MIG-22 | Critério de virada do § 8.2 cumprido e registrado pelo comitê | Ata de virada |
| MIG-23 | Plano de volta ensaiado; reencaminhamento de estudos novos ao PACS antigo durante a janela | Ata do ensaio; teste de reencaminhamento |
| MIG-24 | Arquivo histórico com hash, índice e acesso por perfil, pelo prazo legal | Consulta de amostra por perfil autorizado e não autorizado |
| MIG-25 | Estoque inicial por inventário físico na virada, com lote e validade | Ata do inventário |
