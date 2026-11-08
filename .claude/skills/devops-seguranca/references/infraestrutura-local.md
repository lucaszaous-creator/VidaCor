# Infraestrutura local, rede e ambientes

Fontes: `docs/11-arquitetura-e-infraestrutura.md` §3, §4, §7 e §8, `docs/13-instalacao-e-distribuicao.md`,
`docs/17-seguranca-por-classe-de-ataque.md` B9 e B11, `docs/18-perfis-permissoes-e-rls.md` §7,
`docs/25-plano-do-projeto.md` PREM-06, PREM-16.

## 1. Topologia física (referência, docs/11 §7.1)

| Item | Recomendação | Situação |
|---|---|---|
| Servidor principal | 16 núcleos, 64 GB, 2 NVMe de 1 TB em espelho (sistema e banco), 4 HDD de 8 a 12 TB em ZFS RAIDZ2 (imagens) | A confirmar com inventário e volume de exames |
| Servidor de contingência | Réplica do PostgreSQL por streaming e réplica das imagens | A confirmar |
| Nobreak | Senoidal online, 30 min de autonomia, desligamento automático (NUT) | A confirmar |
| Switch | Gerenciável, com VLANs | A confirmar |
| Firewall | OPNsense ou pfSense, VPN WireGuard de administração | A confirmar |
| Backup externo | Nuvem no Brasil, criptografado no cliente | A confirmar provedor (PREM-16) |

O dimensionamento final depende do inventário (docs/09) e do volume mensal de exames. Qualquer
reaproveitamento de hardware é decisão registrada em `docs/decisoes.md`.

## 2. Rede e VLANs

| VLAN | Quem está | Pode falar com |
|---|---|---|
| Aparelhos DICOM | Eco, USG, RX, TC, conversor | Só Orthanc (portas DICOM), por AE Title e IP cadastrados (SEC-MSG-7) |
| Estações clínicas | Recepção, salas, consultórios, TV | Só o gateway HTTPS (Caddy) |
| Servidores | Núcleo, PostgreSQL, Orthanc, NATS, Keycloak, assinatura, IA, observabilidade | Entre si pelo mínimo necessário, com TLS |
| Administração | TI, VPN WireGuard | Consoles (Grafana, Orthanc, Keycloak, NATS, banco), SSH |
| Wi-Fi de visitantes | Pacientes e acompanhantes | Só internet; nada da rede interna |
| Câmeras | Estacionamento (fase posterior) | Só o serviço de leitura de placa |

Regras:
- Negar por padrão entre VLANs; cada liberação é uma regra com porta, origem, destino e motivo,
  versionada junto da configuração do firewall.
- Consoles **só** na VLAN de administração ou pela VPN (SEC-INF-2). A rede das estações nunca enxerga
  Grafana, Orthanc Explorer, Keycloak admin, NATS monitor ou a porta do PostgreSQL.
- Banco acessível só pelo núcleo e pelos serviços autorizados; nunca por estação nem pelo portal (G1).
- Servidor sem porta aberta para a internet; saída controlada por lista (provedores de PIX, WhatsApp,
  NFS-e, operadoras, assinatura, backup, repositório de versões).

## 3. Plataforma de software (docs/11 §7.2)

| Camada | Escolha |
|---|---|
| Sistema | Debian 12 ou Ubuntu 24.04 LTS (opção Windows Server só se a clínica exigir, docs/13 §2) |
| Arquivos | ZFS: snapshots, compressão, criptografia nativa, *scrub* periódico |
| Execução | Contêineres com Docker Engine e Compose, ou Podman, gerenciados por `systemd`, um stack por ambiente |
| Proxy e TLS | Caddy com certificados de uma CA interna; TLS 1.3 |
| Segredos | SOPS + age no repositório de infraestrutura |
| Identidade | Keycloak (OIDC, 2FA obrigatório para a equipe), console só na VLAN de administração |
| Provisionamento | Ansible declarativo, capaz de reinstalar o servidor do zero e montar a contingência idêntica |

## 4. Contêineres (SEC-INF-1)

Todo serviço roda sem root, com sistema de arquivos somente leitura, sem *capabilities* extras e sem
o socket do Docker. Exemplo (esboço, ajustar usuário e volumes ao serviço real):

```yaml
services:
  nucleo:
    image: registro.local/vidacor/nucleo@sha256:<digest>   # por digest, nunca latest
    user: "10001:10001"
    read_only: true
    cap_drop: [ALL]
    security_opt:
      - no-new-privileges:true
    tmpfs:
      - /tmp:size=64m,mode=1777
    environment:
      VIDACOR_CONFIG: /run/config/nucleo.toml
    secrets:
      - nucleo_db_senha
    networks: [servidores]
    healthcheck:
      test: ["CMD", "/app/nucleo", "health", "--ready"]
      interval: 10s
      timeout: 2s
      retries: 3
    restart: unless-stopped
    deploy:
      resources:
        limits: { cpus: "4", memory: 4g }

secrets:
  nucleo_db_senha:
    file: /run/vidacor/segredos/nucleo_db_senha   # gerado a partir do SOPS na instalação

networks:
  servidores:
    internal: true
```

Revisar sempre: imagem por digest, `user` não zero, `read_only`, `cap_drop: [ALL]`,
`no-new-privileges`, segredo por arquivo (nunca em `environment` em texto puro), rede `internal` para
quem não precisa de saída, limites de recurso, health check.

## 5. Segredos com SOPS e age

- Um arquivo cifrado por ambiente (`infra/segredos/homolog.sops.yaml`, `infra/segredos/producao.sops.yaml`).
- Chaves age por pessoa da operação e uma chave de recuperação guardada fora do servidor, em cofre
  físico da clínica (procedimento **a confirmar** com a clínica).
- Rotação: na saída de uma pessoa, em incidente e na periodicidade definida por tipo de segredo.

Esboço de `.sops.yaml`:

```yaml
creation_rules:
  - path_regex: infra/segredos/producao\.sops\.yaml$
    age: >-
      <chave-publica-age-operacao-1>,
      <chave-publica-age-operacao-2>,
      <chave-publica-age-recuperacao>
  - path_regex: infra/segredos/homolog\.sops\.yaml$
    age: <chave-publica-age-operacao-1>,<chave-publica-age-operacao-2>
```

Nunca versionar o segredo decifrado. A instalação decifra para `/run/vidacor/segredos/` (tmpfs, permissão
restrita ao usuário do serviço).

## 6. Ambientes (docs/11 §7.3)

| Ambiente | Onde | Dados | Regras |
|---|---|---|---|
| `dev` | Máquina do desenvolvedor, Compose | Fictícios (`design/dados-ficticios.md`), DICOM de exemplo | Orthanc e aparelho simulado; nenhum segredo real |
| `homolog` | VM na clínica ou nuvem no Brasil | Fictícios ou anonimizados | Mesma configuração de produção (12-Factor); provedores em sandbox ou fake |
| `producao` | Servidor local | Reais | Acesso só pela VPN de administração; TI sem dado clínico |
| Treinamento | Igual a produção | Fictícios | Exigido pelo docs/16 R12 |

Ensaios de migração com dado real só em instância isolada no servidor da clínica, apagada com registro
(DEC-2026-11-08-22). Nunca em `dev` nem em `homolog`.

## 7. Portal na nuvem (ADR-0011)

- Nuvem no Brasil; recebe só conteúdo publicado (laudo assinado e liberado, documentos).
- Identidade da carga com privilégio mínimo; armazenamento de objetos privado, acessado só por link
  assinado e curto (SEC-INF-5, SEC-DAD-6).
- RLS por paciente no banco do portal como segunda barreira (CLAUDE.md G1, docs/18 §7).
- O servidor local publica para o portal por conexão de saída; o portal não alcança a rede da clínica.

## 8. Instalação (docs/13)

- `infra/installer/server/`: ISO ou script, Ansible e `vidacor-setup` (rede, discos, certificados,
  administradores, dados da clínica, aparelhos por AE Title e IP).
- Pós-instalação automática: health de todos os serviços, C-ECHO com cada aparelho, primeiro backup e
  teste de restauração, painel "Visão da clínica agora".
- Estações registradas por certificado de máquina antes de acessar; BitLocker recomendado à TI.
- Entrega o manual de instalação e o checklist de aceite para a TI da clínica.

## 9. Checklist de mudança de infraestrutura

- [ ] Plano escrito e OK do dono antes de mexer (G6)
- [ ] Mudança em código (`infra/`), revisada em PR, aplicada primeiro em `homolog`
- [ ] Regra de firewall com motivo; nenhum console fora da VLAN de administração
- [ ] Contêiner segue o § 4; segredo segue o § 5
- [ ] Backup antes e plano de volta escrito
- [ ] Componente novo tem ADR (docs/14 A1)
