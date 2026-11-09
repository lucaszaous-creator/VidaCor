# ADR-0022 · Decisões tomadas a partir dos sistemas que a clínica usa (Medware, Mediworks, Medilab) e de sistemas grandes

- **Status:** aceito (regra do dono do produto, 09/11/2026)
- **Data:** 2026-11-09
- **Complementa:** [ADR-0019](0019-decisoes-pelo-mais-profissional-com-lastro.md) (não o substitui)

## Contexto
O dono do produto determinou, ao entregar a continuação das telas a dois agentes: "as decisões que você e o outro
agente devem tomar são sempre baseadas em sistemas grandes de clínica e principalmente em como os sistemas Medware,
Mediworks e Medilab utilizam, pois são eles que o cliente utiliza atualmente". Na mesma mensagem, os agentes passaram
a ter o comando da execução, sem perguntar ao dono a cada escolha.

## Decisão
1. **Referência principal:** em toda escolha de comportamento, campo, fluxo ou nome de tela, o ponto de partida é
   **como a Medware (Clínicas, Laudos Flex, comDICOM, Paciente Conectado, MultiChat), a Mediworks/MediSystem PACS e
   a Medilab (MediClinic RIS, Mediflow, MediWeb, MediAutonotify) fazem hoje**, conforme o
   [levantamento](../03-levantamento-sistemas-atuais.md) e o material público dos fornecedores. A equipe já trabalha
   assim; manter o que ela conhece reduz treino e erro (R1, docs/16).
2. **Referência secundária:** na falta de informação desses três, segue-se a prática de **sistemas grandes de clínica
   e de RIS/PACS** e as normas do setor (TISS/ANS, IHE, DICOM, CFM, LGPD, ONA/JCI).
3. **As seis garantias e as regras clínicas vencem a referência.** Se o sistema atual faz algo que fere G1–G6, docs/16,
   docs/17, docs/18 ou docs/21 (por exemplo, apagar registro, decidir preço na tela, laudo no portal antes de
   assinado, CD com a marca), a VidaCor mantém a **intenção** do comportamento e faz do jeito seguro, registrando a
   diferença.
4. **Lastro da referência:** toda decisão em [`docs/decisoes.md`](../decisoes.md) diz, na coluna "Motivo", qual sistema
   serviu de base e como. Quando não foi possível verificar como o sistema atual faz (sem acesso a ele nem material
   público), escreve-se **"não verificado"** e o motivo, e decide-se pelo item 2, sem inventar.
5. **Os agentes no comando:** os assistentes que receberem o trabalho decidem e executam sem consultar o dono a cada
   passo. Continuam reservados ao dono e ao responsável clínico de segurança apenas os atos de aprovação formal da
   regra (frases de comportamento, risco clínico, ADR, migration, aceite, prontidão), que ficam registrados como
   **"proposta, aguardando aprovação"** sem travar a construção.

## Consequências
Telas e regras mais próximas do que a recepção, o caixa, o faturamento e os médicos já usam; cada diferença em relação
ao sistema atual fica justificada no registro e pode ser revista pelo dono pelo caminho indicado na decisão.
