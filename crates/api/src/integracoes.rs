//! Integrações externas e o que cada uma precisa no ambiente do servidor (DEC-2026-11-08-78).
//! Fonte única para a inicialização (`main.rs`) e para a tela de Configurações. Valores nunca saem
//! daqui: a tela recebe só o NOME de cada variável e se ela está preenchida (segredo não é exibido,
//! nem em parte). Credenciais ficam só no ambiente do servidor (G1: nada no navegador).

use serde::Serialize;
use utoipa::ToSchema;

/// Variável de ambiente de uma integração.
pub struct Variavel {
    pub nome: &'static str,
    pub descricao: &'static str,
    pub obrigatoria: bool,
    pub segredo: bool,
}

/// Integração e suas variáveis.
pub struct Definicao {
    pub id: &'static str,
    pub nome: &'static str,
    pub para_que: &'static str,
    pub variaveis: &'static [Variavel],
    /// Variável que liga o simulado de desenvolvimento (sem valor legal, fiscal ou financeiro).
    pub modo_desenvolvimento: Option<(&'static str, &'static str)>,
}

const fn v(
    nome: &'static str,
    descricao: &'static str,
    obrigatoria: bool,
    segredo: bool,
) -> Variavel {
    Variavel {
        nome,
        descricao,
        obrigatoria,
        segredo,
    }
}

pub const INTEGRACOES: &[Definicao] = &[
    Definicao {
        id: "email",
        nome: "E-mail (SMTP)",
        para_que: "Convite e acesso ao portal do paciente.",
        variaveis: &[
            v("VIDACOR_SMTP_URL", "smtps://usuario:senha@servidor:465 do provedor de e-mail da clínica (em dev, smtp://127.0.0.1:1025 do Mailpit).", true, true),
            v("VIDACOR_EMAIL_REMETENTE", "Remetente, ex.: \"VidaCor Imagem <portal@dominio-da-clinica>\".", true, false),
        ],
        modo_desenvolvimento: None,
    },
    Definicao {
        id: "whatsapp",
        nome: "WhatsApp (Cloud API da Meta)",
        para_que: "Confirmação de horário, laudo pronto, reserva expirada e oferta de vaga.",
        variaveis: &[
            v("VIDACOR_WHATSAPP_NUMERO_ID", "ID do número de telefone no WhatsApp Manager (só dígitos).", true, false),
            v("VIDACOR_WHATSAPP_TOKEN", "Token de usuário do sistema da conta comercial.", true, true),
            v("VIDACOR_WHATSAPP_SEGREDO", "Segredo do app (confere a assinatura dos webhooks recebidos).", true, true),
            v("VIDACOR_WHATSAPP_API_URL", "Opcional; padrão https://graph.facebook.com/v21.0.", false, false),
        ],
        modo_desenvolvimento: Some(("VIDACOR_MENSAGEIRO", "desenvolvimento")),
    },
    Definicao {
        id: "nfse",
        nome: "NFS-e (prefeitura)",
        para_que: "Nota fiscal do atendimento particular, emitida e cancelada pelo outbox.",
        variaveis: &[
            v("VIDACOR_NFSE_PROVEDOR", "nacional (Emissor Nacional da NFS-e) ou o padrão do município.", true, false),
            v("VIDACOR_NFSE_AMBIENTE", "homologacao ou producao.", true, false),
            v("VIDACOR_NFSE_MUNICIPIO_IBGE", "Código IBGE do município (7 dígitos).", true, false),
            v("VIDACOR_NFSE_CNPJ", "CNPJ do prestador (só dígitos).", true, false),
            v("VIDACOR_NFSE_INSCRICAO_MUNICIPAL", "Inscrição municipal do prestador.", true, false),
            v("VIDACOR_NFSE_CODIGO_SERVICO", "Item da lista de serviços (LC 116), ex.: 4.02.", true, false),
            v("VIDACOR_NFSE_CERTIFICADO_ARQUIVO", "Caminho do certificado A1 (.pfx) no servidor, legível só pelo núcleo.", true, false),
            v("VIDACOR_NFSE_CERTIFICADO_SENHA", "Senha do certificado A1.", true, true),
        ],
        modo_desenvolvimento: Some(("VIDACOR_NFSE_PROVEDOR", "desenvolvimento")),
    },
    Definicao {
        id: "pix",
        nome: "PIX (PSP)",
        para_que: "Cobrança e confirmação de pagamento no caixa.",
        variaveis: &[v("VIDACOR_PIX_WEBHOOK_SEGREDO", "Segredo do webhook do PSP.", true, true)],
        modo_desenvolvimento: Some(("VIDACOR_PSP", "desenvolvimento")),
    },
    Definicao {
        id: "assinatura",
        nome: "Assinatura ICP-Brasil",
        para_que: "Assinatura do laudo.",
        variaveis: &[],
        modo_desenvolvimento: Some(("VIDACOR_ASSINADOR", "desenvolvimento")),
    },
    Definicao {
        id: "visualizador",
        nome: "Visualizador de imagens (OHIF)",
        para_que: "Abrir as imagens do estudo a partir do laudo.",
        variaveis: &[v("VIDACOR_VISUALIZADOR_URL", "Endereço público do OHIF.", true, false)],
        modo_desenvolvimento: None,
    },
];

/// Variável como a tela vê: nome e se está preenchida. Nunca o valor.
#[derive(Serialize, ToSchema, Debug)]
pub struct VariavelVista {
    pub nome: String,
    pub descricao: String,
    pub obrigatoria: bool,
    pub segredo: bool,
    pub preenchida: bool,
}

#[derive(Serialize, ToSchema, Debug)]
pub struct IntegracaoVista {
    pub id: String,
    pub nome: String,
    pub para_que: String,
    /// em_uso, desenvolvimento, incompleta, aguardando_adaptador ou nao_configurada.
    pub situacao: String,
    /// O que falta, em frase, para a tela.
    pub o_que_falta: Option<String>,
    pub variaveis: Vec<VariavelVista>,
    /// Itens desta integração parados no outbox (não se perdem; saem quando ela estiver pronta).
    pub pendentes_no_outbox: i64,
}

/// Lê se a variável está preenchida no ambiente do processo (só a presença, nunca o valor).
#[must_use]
pub fn preenchida(nome: &str) -> bool {
    std::env::var(nome).is_ok_and(|v| !v.trim().is_empty())
}

fn valor_e(nome: &str, esperado: &str) -> bool {
    std::env::var(nome).is_ok_and(|v| v.trim() == esperado)
}

/// Situação de uma integração. `ativa` diz se o núcleo ligou um adaptador real para ela.
#[must_use]
pub fn situacao(d: &Definicao, ativa: bool) -> (String, Option<String>) {
    if let Some((var, val)) = d.modo_desenvolvimento {
        if valor_e(var, val) {
            return ("desenvolvimento".into(), Some("Simulado de desenvolvimento ligado: sem valor legal, fiscal ou financeiro. Nunca em produção.".into()));
        }
    }
    if ativa {
        return ("em_uso".into(), None);
    }
    let faltam: Vec<&str> = d
        .variaveis
        .iter()
        .filter(|x| x.obrigatoria && !preenchida(x.nome))
        .map(|x| x.nome)
        .collect();
    let alguma = d.variaveis.iter().any(|x| preenchida(x.nome));
    if d.variaveis.is_empty() {
        return ("nao_configurada".into(), Some("O adaptador do serviço de assinatura ICP-Brasil ainda não foi escolhido (ADR-0008).".into()));
    }
    match (faltam.is_empty(), alguma) {
        (true, _) => (
            "aguardando_adaptador".into(),
            Some("Configuração completa, mas o adaptador deste provedor ainda não está no núcleo. Os itens ficam no outbox até ele entrar.".into()),
        ),
        (false, true) => ("incompleta".into(), Some(format!("Falta preencher: {}.", faltam.join(", ")))),
        (false, false) => ("nao_configurada".into(), Some(format!("Preencha no ambiente do servidor: {}.", faltam.join(", ")))),
    }
}

/// Tipos do outbox de cada integração (para contar o que está parado esperando por ela).
#[must_use]
pub fn tipos_do_outbox(id: &str) -> &'static [&'static str] {
    match id {
        "email" => &["email.convite_portal"],
        "whatsapp" => &[
            "aviso.confirmacao",
            "aviso.laudo_pronto",
            "aviso.reserva_expirada",
            "aviso.oferta_vaga",
        ],
        "nfse" => &["nfse.emitir", "nfse.cancelar"],
        "pix" => &["pix.criar_cobranca", "pix.estornar"],
        _ => &[],
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn segredos_marcados_e_nomes_unicos() {
        let mut nomes: Vec<&str> = INTEGRACOES
            .iter()
            .flat_map(|d| d.variaveis.iter().map(|x| x.nome))
            .collect();
        let total = nomes.len();
        nomes.sort_unstable();
        nomes.dedup();
        assert_eq!(nomes.len(), total, "variável repetida");
        for s in [
            "VIDACOR_WHATSAPP_TOKEN",
            "VIDACOR_SMTP_URL",
            "VIDACOR_NFSE_CERTIFICADO_SENHA",
            "VIDACOR_WHATSAPP_SEGREDO",
        ] {
            let x = INTEGRACOES
                .iter()
                .flat_map(|d| d.variaveis.iter())
                .find(|x| x.nome == s)
                .unwrap();
            assert!(x.segredo, "{s} é segredo");
        }
        // Nenhuma variável de integração vai para o navegador.
        assert!(nomes.iter().all(|n| !n.starts_with("VITE_")));
    }

    #[test]
    fn situacao_sem_nada_incompleta_e_completa_sem_adaptador() {
        const D: Definicao = Definicao {
            id: "teste",
            nome: "Teste",
            para_que: "teste",
            variaveis: &[
                v("VIDACOR_TESTE_SITUACAO_A", "a", true, false),
                v("VIDACOR_TESTE_SITUACAO_B", "b", true, true),
            ],
            modo_desenvolvimento: None,
        };
        assert_eq!(situacao(&D, false).0, "nao_configurada");
        std::env::set_var("VIDACOR_TESTE_SITUACAO_A", "x");
        let (s, falta) = situacao(&D, false);
        assert_eq!(s, "incompleta");
        assert!(falta.unwrap().contains("VIDACOR_TESTE_SITUACAO_B"));
        std::env::set_var("VIDACOR_TESTE_SITUACAO_B", "y");
        assert_eq!(situacao(&D, false).0, "aguardando_adaptador");
        assert_eq!(situacao(&D, true).0, "em_uso");
    }
}
