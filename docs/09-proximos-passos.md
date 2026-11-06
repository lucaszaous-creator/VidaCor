# 09 · Próximos passos

## Com o cliente

1. Apresentar o mockup (rodada 3) e colher ajustes finos.
2. Confirmar o quarto sistema que a clínica usa e o que cada sistema faz no dia a dia
   (prints ajudam muito), além de quanto paga hoje em cada um.
3. Inventário dos **aparelhos** (marca, modelo, suporte a DICOM/worklist/SR) e
   do **servidor** (CPU, RAM, disco, sistema operacional).
4. Volume mensal de exames e de imagens (dimensionar disco e backup).
5. Estacionamento: cobrar ou só controlar, quantas entradas, se já tem cancela,
   mensalistas; instalar **uma câmera de teste** por um dia e uma noite.
6. Arquivo original do **logo**, dados da clínica (endereço, CNPJ, responsável
   técnico e CRM), modelos de laudo e textos de preparo validados.
7. Exportação de dados da Medware e da Medilab para a migração.
8. Avaliação regulatória (Anvisa) do visualizador usado para diagnóstico.

## Com o time

1. Decidir a arquitetura ([07](07-arquitetura-proposta.md)): reaproveitar a suíte
   Clinica (recomendado) e fixar a stack de imagens (Orthanc, OHIF, DCMTK).
2. Montar o ambiente de laboratório: Orthanc + worklist + um aparelho simulado
   (DICOM de exemplo) + OHIF.
3. Prova de conceito do estacionamento: fast-alpr lendo gravação real, registrando
   entrada e saída e calculando a permanência.
4. Quebrar o escopo ([04](04-escopo-funcional.md)) em entregas:
   - **Entrega 1**: agenda por sala, recepção, caixa, cadastro e portal (convite
     automático).
   - **Entrega 2**: sala de exame, worklist, laudo com SR, fluxo de laudos, PACS e
     visualizador, impressos.
   - **Entrega 3**: faturamento de imagem, financeiro, estoque, WhatsApp e NPS,
     indicadores.
   - **Entrega 4**: estacionamento, apps, Meus Exames Bebê, telemedicina, TV.
5. Plano de migração e de virada (rodar em paralelo com os sistemas atuais).
