# 02 · Estacionamento com leitura de placa

## Objetivo

Identificar a placa na **entrada** e na **saída**, calcular o **tempo de
permanência** e, conforme o cliente decidir, controlar **cancela**, emitir
**ticket** e **cobrar**. Tudo roda no **servidor local** da clínica.

## Repositórios avaliados (GitHub, nov/2026)

| Repositório | Estrelas | Licença | Avaliação |
|---|---|---|---|
| [ankandrew/fast-alpr](https://github.com/ankandrew/fast-alpr) | 856 | MIT | **Recomendado.** Python, CPU, detecção + OCR em poucas linhas |
| [ankandrew/fast-plate-ocr](https://github.com/ankandrew/fast-plate-ocr) | 805 | MIT | OCR usado pelo fast-alpr; modelo `global` inclui **Brazil** (conferido no `plate_config.yaml`); dá para treinar com fotos locais |
| [faisalthaheem/open-lpr](https://github.com/faisalthaheem/open-lpr) | 236 | Apache-2.0 | Sistema pronto com web/API/Docker, bom para demo |
| [blakeblackshear/frigate](https://github.com/blakeblackshear/frigate) | 37k | MIT | NVR com LPR e placas conhecidas; não faz entrada/saída sozinho |
| [luancesarcode/mercosul-anpr](https://github.com/luancesarcode/mercosul-anpr) | 10 | MIT | Focado em placa brasileira/Mercosul; usa YOLO (verificar AGPL da Ultralytics) |
| [openalpr/openalpr](https://github.com/openalpr/openalpr) | 11k | AGPL-3.0 | Parado e com licença ruim para uso comercial: evitar |
| [evintran1508/parking-management-system](https://github.com/evintran1508/parking-management-system) | 4 | **sem licença** | O mais completo (cancela, preço, pagamento), mas não pode ser copiado; serve só de referência |
| [parkpow/deep-license-plate-recognition](https://github.com/parkpow/deep-license-plate-recognition) | 672 | MIT (SDK) | Plate Recognizer: API comercial paga, alternativa de alta precisão |

**Conclusão:** reaproveitar a leitura de placa (fast-alpr) e construir a gestão.

## Peças da solução

| Parte | Solução |
|---|---|
| Leitura da placa | fast-alpr no servidor local (ou câmera que já entrega a leitura pronta) |
| Entrada, saída e permanência | Nosso sistema (módulo Estacionamento) |
| Cancela | Contato de relé por módulo de relé de rede ou saída de alarme da câmera |
| Ticket / totem | Impressora térmica + [python-escpos](https://github.com/python-escpos/python-escpos) (MIT) |
| PIX | API de banco ou intermediador (Mercado Pago, Efí, Asaas…) |
| Cartão | Maquininha integrada (fase posterior) |

```
ENTRADA: câmera lê placa → grava placa + horário → (ticket) → abre cancela
SAÍDA:   câmera lê placa → calcula tempo e valor → paga / isento (exame) → abre cancela
```

## Câmera: o que realmente importa

1. **Tamanho da placa na imagem**: 130–200 px de largura, com lente varifocal
   (2,8–12 mm) fechada na faixa. Lente aberta de 2,8 mm não serve.
2. **Obturador rápido** (1/1000 s ou mais) para o carro em movimento.
3. **Infravermelho e exposição** ajustados para a placa refletiva, que estoura
   com IR comum.
4. **WDR verdadeiro** (≥ 120 dB) contra o farol.
5. **Posição**: até 30° de lado e de cima, a 1–1,5 m de altura, a 3–6 m do ponto
   de leitura.

Com **cancela** o carro para, então uma câmera comum boa (2–4 MP, varifocal, WDR)
costuma funcionar. Uma câmera própria para leitura de placa é o mais seguro à
noite. **Antes de comprar: testar uma câmera um dia e uma noite e medir a taxa
de acerto.**

## Fases

1. Monitorar: painel com placa, entrada, saída e permanência.
2. Cancela automática, com liberação manual no painel.
3. Cobrança: tabela, PIX, ticket ou totem.

## Integração com a clínica (diferencial)

- **Validação pelo exame**: placa de paciente com check-in é isentada ou
  descontada automaticamente (aparece na Recepção, no Caixa e no Início).
- Placa no cadastro do paciente e no comprovante de agendamento.

## Pontos de atenção

- Placa antiga, placa Mercosul e placa de moto (duas linhas).
- Sempre ter correção manual.
- **LGPD**: placa com horário é dado pessoal; definir prazo de retenção e quem
  acessa.

## Perguntas em aberto

Cobrar ou só controlar? Quantas entradas/saídas? Já tem cancela (marca)?
Mensalistas (médicos e funcionários)? Configuração do servidor?
