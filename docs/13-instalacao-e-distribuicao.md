# 13 · Instalação e distribuição nas máquinas da clínica

> **Status:** proposta (v0.1, nov/2026). Decisão no
> [ADR-0013](adr/0013-distribuicao-instaladores.md).

O VidaCor é **instalável**: um instalador para o servidor e outro para as
estações (recepção, consultórios, salas de exame, TV). Atualizações são
assinadas, distribuídas pelo próprio servidor da clínica e funcionam sem
internet nas estações.

## 1. O que é instalado onde

| Máquina | O que roda | Instalador |
|---|---|---|
| **Servidor principal** | Núcleo VidaCor (Rust), PostgreSQL, Orthanc, NATS, Keycloak, serviço de assinatura, workers de IA, observabilidade, repositório de atualizações | **VidaCor Server** (ver §2) |
| **Servidor de contingência** | Réplicas (banco e imagens) e os mesmos serviços em espera | VidaCor Server, modo `replica` |
| **Estações de trabalho** (Windows) | **VidaCor Desktop** (Tauri): telas, impressora de etiquetas, scanner, tablet de assinatura, leitor de código de barras | **MSI** assinado |
| **TV da sala de espera** | VidaCor Desktop em **modo quiosque** (só a TV de chamada) | MSI com perfil `tv` |
| **Qualquer navegador da rede** | Aplicação web (mesmas telas, sem periféricos) | Nada a instalar |

Rust compila **nativamente** para Windows e Linux. Os binários do núcleo não
precisam de runtime instalado, o que simplifica instalação e atualização.

## 2. VidaCor Server

### Opção A — Linux (recomendada)
- Imagem/ISO de instalação baseada em **Debian 12** com ZFS, ou script de provisionamento para uma máquina já instalada.
- Instalação guiada (`vidacor-setup`): rede e VLANs, discos (espelho e RAIDZ2), certificados internos, contas de administrador, dados da clínica, importação dos aparelhos (AE Title/IP).
- Serviços em contêineres gerenciados por `systemd` (Compose/Podman), com *healthcheck* e reinício automático.
- Provisionamento declarativo e reproduzível (Ansible) para reinstalar o servidor do zero e montar o servidor de contingência idêntico.

### Opção B — Windows Server (se a clínica exigir)
- Núcleo VidaCor, NATS e workers como **serviços do Windows** (binários nativos).
- PostgreSQL e Orthanc com os instaladores oficiais para Windows.
- Menos recursos de armazenamento (sem ZFS); backup e réplica configurados pelo instalador.

### Pós-instalação automática
Verificação de saúde de todos os serviços, C-ECHO com cada aparelho cadastrado,
primeiro backup e teste de restauração, painel "Visão da clínica agora" no Grafana.

## 3. VidaCor Desktop (estações)

- Aplicativo **Tauri** (Rust + WebView2 do Windows), leve (dezenas de MB, contra centenas de um Electron).
- **MSI assinado** com certificado de assinatura de código. Instalação silenciosa para TI: `msiexec /i VidaCor.msi /qn SERVER=https://vidacor.local PERFIL=recepcao`.
- **Perfis por máquina**: `recepcao`, `consultorio`, `sala-exame`, `laudo`, `caixa`, `tv`. O perfil define as telas iniciais e os periféricos habilitados, mas o acesso continua sendo decidido pelo login do usuário.
- **Periféricos**: impressora térmica de etiquetas (ZPL/ESC-POS), impressora A4, scanner/câmera para documentos e carteirinha, leitor de código de barras, tablet de assinatura do paciente.
- **Resiliência**: se a estação perder a rede por segundos, a tela avisa e reconecta sozinha; nada fica salvo só na máquina (dado de saúde não fica em disco local).
- Distribuição em massa por **GPO/Intune** ou **winget** (repositório interno).

## 4. Atualizações

| Item | Como funciona |
|---|---|
| Canal | A CI publica versões assinadas; o servidor da clínica **baixa** (conexão de saída) e vira o repositório local |
| Estações | O Desktop consulta o **servidor local**, não a internet, e se atualiza com o *updater* assinado do Tauri, fora do horário ou quando o usuário sair |
| Servidor | Atualização em janela de manutenção, com **migração de banco aditiva**, backup imediatamente antes e **rollback** em um comando |
| Compatibilidade | Desktop e servidor negociam versão da API; uma estação desatualizada continua funcionando por um prazo definido |
| Canais | `estavel` (produção) e `piloto` (máquinas escolhidas recebem antes) |
| Integridade | Pacotes assinados e verificados antes de instalar; SBOM publicado por versão |

## 5. Segurança da instalação

- Instaladores e atualizações **assinados**; a estação recusa pacote sem assinatura válida.
- Cada estação é **registrada** no servidor (certificado de máquina) antes de acessar.
- Disco das estações com BitLocker (recomendação para a TI da clínica).
- Sem porta aberta para a internet no servidor; suporte remoto por **VPN WireGuard** com acesso registrado.

## 6. Custos associados

| Item | Observação |
|---|---|
| Certificado de assinatura de código (Windows) | Necessário para o MSI não ser bloqueado pelo SmartScreen; anual |
| Licença do Windows Server | Só na opção B |
| Hardware | Ver [11 · §7.1](11-arquitetura-e-infraestrutura.md#71-topologia-física) |

## 7. Entregáveis de engenharia

1. `infra/installer/server/` — ISO/script + Ansible + `vidacor-setup`.
2. `apps/desktop/` — Tauri com perfis e geração do MSI na CI.
3. Repositório de atualizações no servidor + *updater* assinado.
4. Manual de instalação para a TI da clínica e checklist de aceite.
