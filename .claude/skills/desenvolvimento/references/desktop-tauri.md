# Desktop Tauri e periféricos

Fontes: ADR-0005 e ADR-0013; `docs/13-instalacao-e-distribuicao.md`; `docs/17-seguranca-por-classe-de-ataque.md` B5;
`docs/20-etiquetas-e-rastreio.md`; `docs/23-modo-contingencia.md` §6.

## 1. Papel do desktop

O **VidaCor Desktop** (`apps/desktop`) é uma casca Tauri sobre a mesma aplicação web. Existe para o que o navegador não
faz bem: periféricos do balcão e das salas e o modo quiosque da TV. **Não tem regra de negócio** e **não decide
permissão**: o servidor decide (SEC-DSK-3).

## 2. Perfis por máquina

Instalação silenciosa por MSI assinado com `SERVER` e `PERFIL` (`docs/13` §3):

| Perfil | Uso | Periféricos típicos |
|---|---|---|
| `recepcao` | Balcão, check-in | Impressora de etiquetas, leitor de código de barras, scanner de documento, tablet de assinatura |
| `caixa` | Recebimentos | Impressora A4 ou térmica de recibo |
| `consultorio` | Atendimento | Impressora A4 |
| `sala-exame` | Execução do exame | Leitor de código de barras (bipe da pulseira) |
| `laudo` | Laudar | Monitores, ditado (F9) |
| `tv` | Painel de chamada em quiosque | Nenhum |

O perfil define tela inicial e periféricos habilitados; **o acesso continua decidido pelo login do usuário**.

## 3. Segurança (SEC-DSK-1 a 6)

- **Capacidades mínimas** por janela: só os comandos necessários; nenhum acesso amplo a arquivo, shell ou rede (SEC-DSK-1).
- O webview carrega **só** a aplicação empacotada ou o servidor VidaCor configurado; conteúdo remoto nunca tem acesso à
  ponte nativa (SEC-DSK-2).
- Todo argumento que chega a um comando nativo é validado (SEC-DSK-3).
- Credenciais locais no cofre do sistema operacional, nunca em arquivo de texto; limpeza no logout e na troca de usuário (SEC-DSK-4).
- Atualizador e instalador **assinados**, verificação antes de aplicar, sem downgrade para versão vulnerável (SEC-DSK-5).
  As estações se atualizam pelo **servidor local**, não pela internet (`docs/13` §4).
- Temporários (PDF, etiqueta) com permissão restrita e apagados, sem corrida entre verificar e abrir (SEC-DSK-6).
- **Nenhum dado de saúde fica em disco local.** Queda de rede por segundos: a tela avisa e reconecta.

Exemplo de comando nativo estreito (ilustrativo; conferir a API do Tauri na versão adotada):

```rust
// PSEUDOCÓDIGO: o comando só imprime um documento já gerado e assinado pelo servidor
#[tauri::command]
async fn imprimir_etiqueta(etiqueta_id: String) -> Result<(), ErroImpressao> {
    let id = Uuid::parse_str(&etiqueta_id).map_err(|_| ErroImpressao::IdInvalido)?;
    // Busca o ZPL pronto no servidor; o servidor confere permissão e registra a impressão no lastro.
    let zpl = cliente_servidor().etiqueta_zpl(id).await?;
    impressora_configurada()?.enviar(&zpl).await
}
```

O comando **não monta** o conteúdo da etiqueta: o servidor gera, numera e registra. Isso preserva o código único, a
reimpressão com motivo e a invalidação da anterior (`docs/20`).

## 4. Periféricos

| Periférico | Regra |
|---|---|
| Impressora térmica de etiquetas (ZPL/ESC-POS) | Conteúdo gerado pelo servidor; etiqueta só com nome, nascimento e códigos |
| Leitor de código de barras | Bipe da pulseira na sala envia o código ao servidor, que confere a sala e **bloqueia** o exame se for de outra sala |
| Scanner ou câmera | Documento enviado ao servidor com hash; nada guardado localmente |
| Tablet de assinatura do paciente | Assinatura vinculada ao documento e à versão do termo, com hash |
| Impressora A4 | Laudo, recibo e documentos em PDF gerados pelo servidor |

## 5. Modo quiosque da TV

- Perfil `tv` sem login de pessoa; credencial do dispositivo com escopo só do painel de chamada.
- Mostra só senha, primeiro nome e iniciais, e sala (`docs/18` §6 item 8).
- Tamanho fixo de 1280×720 (exceção à regra de "cabe na janela").

## 6. Contingência (`docs/23` §6)

- Cópia local da agenda do dia nas estações (cifrada, só leitura, expira no dia, login recente mais PIN) foi registrada em
  DEC-2026-11-08-14 e **exige ADR antes de construir: ADR pendente**. Até o ADR ser aprovado, nada de dado de paciente em
  disco da estação.
- O kit das 06:00 (PDF da agenda e fichas, faixas numeradas) é gerado pelo servidor com numeração reservada no banco.

## 7. Compatibilidade e distribuição

- Desktop e servidor negociam versão da API; estação desatualizada funciona por prazo definido.
- Canais `piloto` e `estavel`; distribuição por GPO/Intune ou winget interno.
- Testes: comandos nativos com argumentos inválidos, permissão negada pelo servidor, perda de rede, impressão repetida
  (não duplica registro), atualização com pacote de assinatura inválida (recusada).
