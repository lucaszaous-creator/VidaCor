# 01 · Histórico da conversa e decisões

Resumo, em ordem, do que foi pedido, pesquisado e decidido até a aprovação da
terceira rodada de mockups (6/nov/2026).

## 1. Reunião com o cliente: estacionamento com leitura de placa

**Pedido:** um sistema de estacionamento que identifique a placa e quanto tempo
o veículo ficou dentro. Talvez já exista algo pronto no GitHub.

**O que encontramos** (detalhes em [02](02-estacionamento-leitura-de-placa.md)):
- Para **ler a placa** há bibliotecas maduras e com licença boa. A recomendada é
  **fast-alpr** (MIT), cujo modelo de leitura inclui placas do **Brasil**.
- **Sistema de estacionamento completo pronto** não existe com licença que
  permita reaproveitar. O mais próximo (eParking) não declara licença e usa YOLOv8
  (AGPL). Decisão: **reaproveitar a leitura de placa e construir a gestão**.

## 2. A câmera importa

Dúvida do time: "toda câmera enxerga a placa, basta infravermelho e boa
qualidade?". Esclarecimento: **a câmera importa muito, mas "boa" não é
resolução**. O que manda é o tamanho da placa na imagem (lente varifocal,
130–200 px de largura de placa), obturador rápido, infravermelho que não estoura
a placa refletiva, WDR para farol, ângulo ≤ 30°. **Com cancela o carro para**,
então uma câmera comum boa costuma funcionar. A recomendação é testar um dia e
uma noite antes de comprar.

## 3. Servidor local, totem, cancela e cobrança

O cliente tem servidor local. Proposta em fases: (1) só monitorar
entrada/saída/permanência, (2) cancela por relé, (3) cobrança (PIX, ticket).
Peças prontas: fast-alpr, python-escpos (impressora térmica), módulo de relé de
rede, API PIX. Ideia de diferencial: **isentar paciente com exame** integrando com
a agenda da clínica.

## 4. Pedido maior: um sistema próprio para a clínica

O cliente usa hoje quatro sistemas. Depois de pesquisar e confirmar os nomes
corretos com o time: **Medware**, **Mediworks** e **Medilab**, mais um conversor
de imagem para DICOM (a Medware oferece o comDICOM, a Medilab o MediPrint DICOM).
Levantamento completo em [03](03-levantamento-sistemas-atuais.md).

Descoberta importante: o nosso repositório **Clinica** (suíte da Sem Dor, .NET 8
+ PostgreSQL) já cobre de 60% a 70% da gestão: agenda, recepção, prontuário,
**TISS completo**, financeiro, estoque, WhatsApp e assinatura ICP-Brasil. Falta o
mundo da imagem: laudo, aparelhos, PACS, visualizador e portal.

A clínica é a **VidaCor Imagem** (logo e fachada enviados pelo time). Exames:
ecocardiograma, Doppler de carótidas, mais de 50 USGs (obstétricas e diversas),
teste ergométrico, raio X, tomografia e MAPA.

## 5. Mockups — rodada 1 (reprovada no visual)

Sete telas simples: recepção, laudo, cadastro, **totem**, portal no celular,
laudo e preparo em A4. Feedback do cliente:
- **Totem desnecessário.**
- **Interface amadora.** Foram enviados modelos de UI: dashboards de saúde com
  cartões brancos, números grandes e ícones em tons pastel, um app com render 3D
  e uma barra lateral no estilo "Ward".
- Pedido de **mapa corporal 3D**.
- **Todas as funções** dos sistemas atuais precisam estar nos mockups.

## 6. Mockups — rodada 2 (aprovada no caminho)

Refeito do zero com design system próprio ([05](05-design-system.md)): 22 telas e
o menu lateral, cobrindo todas as funções levantadas. O Higgsfield estava sem
créditos, então o mapa 3D foi renderizado aqui com three.js, a partir de um
manequim. As imagens de exame vieram de **DICOM de exemplo reais**.

O primeiro carregamento falhou porque as telas ainda não existiam e por um
detalhe no menu compartilhado. Corrigido.

Feedback: "o caminho está perfeito, genial", mas:
- O mapa 3D parecia **boneco de avatar**. Querem **realismo**.
- Havia **telas espremidas e responsividade ruim**: Configurações, Comunicação e
  a fila de check-in.
- Faltava a **área de atendimento médico**: evolução, prescrição, documentos.
- A **fila de check-in estava feia** e sem **horários livres**.
- Faltava um **caixa do particular** com gestão profissional.
- Ainda havia funções a integrar nos outros módulos.

## 7. Mockups — rodada 3 (aprovada: "Perfeito!")

- **Anatomia realista**: modelos do **BodyParts3D** (pele translúcida, esqueleto,
  coração, aorta, carótidas, pulmões, fígado, rins, estômago, intestino e bexiga)
  em três vistas: corpo inteiro, foco cardio e foco abdome. Fundo escuro, como nos
  visualizadores médicos, com pontos clicáveis por exame.
- **Atendimento médico** (tela nova): SOAP, CID-10, prescrição Memed, pedidos com
  guia SADT, documentos assinados, sinais vitais e anatomia.
- **Caixa do particular** (tela nova): abertura, sangria, suprimento, fechamento
  com conferência, pagamento dividido, desconto com autorização, NFS-e.
- **Recepção refeita**: check-in em cartões com uma próxima ação, linha do dia com
  horários livres e próximos horários por exame.
- Layout sem telas espremidas: helpers `.split`, grades com largura mínima,
  tabelas com rolagem horizontal.
- Funções extras em todos os módulos (veja [04](04-escopo-funcional.md)).

## 8. Esta etapa

Pedido: **documentar as configurações e a conversa**, levar o projeto para um
repositório próprio (este, `lucaszaous-creator/VidaCor`) e atualizar o README.
O conteúdo começou numa branch do repositório Almoxerifado, que segue intacto. Depois
disso seguem os próximos passos ([09](09-proximos-passos.md)).
