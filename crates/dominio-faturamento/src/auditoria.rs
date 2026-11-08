//! Auditoria antes do faturamento (docs/22 E5). Cada operadora tem um conjunto de regras com versão
//! (ex.: `AUD-UNIMED-2026.1`); a conta grava a versão que a conferiu e a mudança de versão não altera
//! auditorias antigas. Pendência que "bloqueia" ou pede "corrigir antes" deixa a guia fora do lote;
//! "atenção" só avisa.
//!
//! Os conjuntos abaixo são o primeiro corte, montado a partir do mockup Auditoria-Faturamento e do
//! padrão TISS. **A confirmar com o analista TISS e cada manual de operadora** (DEC-2026-11-08-65).

use crate::guia::{cid_valido, Centavos, GuiaFaturavel};
use chrono::NaiveDate;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Efeito {
    /// Precisa de ação externa (operadora, médico): fora do lote.
    Bloqueia,
    /// Corrigível pelo faturamento: fora do lote até corrigir.
    CorrigirAntes,
    /// Só aviso: a guia entra no lote.
    Atencao,
}

impl Efeito {
    #[must_use]
    pub fn impede_lote(self) -> bool {
        !matches!(self, Self::Atencao)
    }

    #[must_use]
    pub fn codigo(self) -> &'static str {
        match self {
            Self::Bloqueia => "bloqueia",
            Self::CorrigirAntes => "corrigir_antes",
            Self::Atencao => "atencao",
        }
    }
}

/// Regras conhecidas. O código é estável (vai para o banco e para a tela).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodigoRegra {
    OperadoraSemContrato,
    GuiaForaDoPadrao,
    SenhaVencida,
    ProcedimentoNaoAutorizado,
    SolicitanteIncompleto,
    ProcedimentoSemPreco,
    ValorDiferenteDaTabela,
    LaudoNaoAssinado,
    CidAusente,
    AssinaturaBeneficiarioAusente,
    PrazoDeEnvioVencido,
    PrazoDeEnvioProximo,
}

impl CodigoRegra {
    #[must_use]
    pub fn codigo(self) -> &'static str {
        match self {
            Self::OperadoraSemContrato => "operadora-sem-contrato",
            Self::GuiaForaDoPadrao => "guia-fora-do-padrao",
            Self::SenhaVencida => "senha-vencida",
            Self::ProcedimentoNaoAutorizado => "procedimento-nao-autorizado",
            Self::SolicitanteIncompleto => "solicitante-incompleto",
            Self::ProcedimentoSemPreco => "procedimento-sem-preco",
            Self::ValorDiferenteDaTabela => "valor-diferente-da-tabela",
            Self::LaudoNaoAssinado => "laudo-nao-assinado",
            Self::CidAusente => "cid-ausente",
            Self::AssinaturaBeneficiarioAusente => "assinatura-beneficiario-ausente",
            Self::PrazoDeEnvioVencido => "prazo-de-envio-vencido",
            Self::PrazoDeEnvioProximo => "prazo-de-envio-proximo",
        }
    }

    /// Título curto, como no mockup.
    #[must_use]
    pub fn titulo(self) -> &'static str {
        match self {
            Self::OperadoraSemContrato => "Operadora sem contrato cadastrado",
            Self::GuiaForaDoPadrao => "Dados da guia fora do padrão TISS",
            Self::SenhaVencida => "Senha vencida antes da data do exame",
            Self::ProcedimentoNaoAutorizado => "Procedimento diferente do autorizado",
            Self::SolicitanteIncompleto => "Dados do médico solicitante incompletos",
            Self::ProcedimentoSemPreco => "Procedimento sem preço na tabela da operadora",
            Self::ValorDiferenteDaTabela => "Valor diferente da tabela vigente",
            Self::LaudoNaoAssinado => "Laudo não assinado",
            Self::CidAusente => "CID-10 obrigatório ausente",
            Self::AssinaturaBeneficiarioAusente => "Assinatura do beneficiário ausente",
            Self::PrazoDeEnvioVencido => "Prazo de envio vencido",
            Self::PrazoDeEnvioProximo => "Prazo de envio vence em até 7 dias",
        }
    }

    /// O que fazer (R8: a mensagem diz o que fazer).
    #[must_use]
    pub fn o_que_fazer(self) -> &'static str {
        match self {
            Self::OperadoraSemContrato => "Cadastre o contrato da operadora (registro ANS, código do prestador e CNES) e confira de novo.",
            Self::GuiaForaDoPadrao => "Confira número, senha e carteirinha na guia autorizada; o TISS aceita até 20 caracteres.",
            Self::SenhaVencida => "Peça prorrogação da senha à operadora.",
            Self::ProcedimentoNaoAutorizado => "Corrija o código ou peça nova senha à operadora.",
            Self::SolicitanteIncompleto => "Preencha conselho, número, UF e CBO do solicitante a partir do pedido médico.",
            Self::ProcedimentoSemPreco => "Inclua o código na tabela da operadora e confira de novo.",
            Self::ValorDiferenteDaTabela => "Reaplique a tabela vigente na correção da guia.",
            Self::LaudoNaoAssinado => "Avise o médico responsável pelo laudo.",
            Self::CidAusente => "Preencha o CID a partir do pedido médico.",
            Self::AssinaturaBeneficiarioAusente => "Colete a assinatura no próximo contato ou pelo portal e registre na correção da guia.",
            Self::PrazoDeEnvioVencido => "Fale com a operadora antes de enviar: o envio fora do prazo costuma ser glosado.",
            Self::PrazoDeEnvioProximo => "Inclua no próximo lote desta operadora.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Regra {
    pub codigo: CodigoRegra,
    pub efeito: Efeito,
    /// De onde veio a regra: padrão TISS, manual do prestador, contrato, regra da clínica, glosa recebida.
    pub origem: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ConjuntoDeRegras {
    pub versao: &'static str,
    pub nome: &'static str,
    pub regras: &'static [Regra],
}

const fn r(codigo: CodigoRegra, efeito: Efeito, origem: &'static str) -> Regra {
    Regra {
        codigo,
        efeito,
        origem,
    }
}

use CodigoRegra as C;
use Efeito as E;

/// Regras que valem para toda operadora (padrão TISS e regras da clínica).
const BASE: [Regra; 10] = [
    r(C::OperadoraSemContrato, E::Bloqueia, "regra da clínica"),
    r(C::GuiaForaDoPadrao, E::Bloqueia, "padrão TISS"),
    r(C::SenhaVencida, E::Bloqueia, "padrão TISS"),
    r(C::ProcedimentoNaoAutorizado, E::Bloqueia, "padrão TISS"),
    r(C::SolicitanteIncompleto, E::CorrigirAntes, "padrão TISS"),
    r(C::ProcedimentoSemPreco, E::Bloqueia, "contrato"),
    r(C::ValorDiferenteDaTabela, E::CorrigirAntes, "contrato"),
    r(C::LaudoNaoAssinado, E::Bloqueia, "regra da clínica"),
    r(C::PrazoDeEnvioVencido, E::Bloqueia, "contrato"),
    r(C::PrazoDeEnvioProximo, E::Atencao, "contrato"),
];

const fn com<const N: usize>(extra: Regra) -> [Regra; N] {
    let mut todas = [extra; N];
    let mut i = 0;
    while i < BASE.len() {
        todas[i] = BASE[i];
        i += 1;
    }
    todas
}

static TISS: [Regra; 10] = BASE;
static UNIMED: [Regra; 10] = BASE;
static SULAMERICA: [Regra; 11] = com(r(C::CidAusente, E::CorrigirAntes, "manual do prestador"));
static BRADESCO: [Regra; 11] = com(r(
    C::AssinaturaBeneficiarioAusente,
    E::CorrigirAntes,
    "manual do prestador",
));

/// Conjuntos vigentes. Versão nova entra aqui com número novo; a anterior continua para reauditar o
/// histórico (nada é apagado).
pub static CONJUNTOS: [ConjuntoDeRegras; 4] = [
    ConjuntoDeRegras {
        versao: "AUD-TISS-2026.1",
        nome: "Padrão TISS (operadora sem regras próprias)",
        regras: &TISS,
    },
    ConjuntoDeRegras {
        versao: "AUD-UNIMED-2026.1",
        nome: "Unimed",
        regras: &UNIMED,
    },
    ConjuntoDeRegras {
        versao: "AUD-SULAMERICA-2026.1",
        nome: "SulAmérica",
        regras: &SULAMERICA,
    },
    ConjuntoDeRegras {
        versao: "AUD-BRADESCO-2026.1",
        nome: "Bradesco Saúde",
        regras: &BRADESCO,
    },
];

/// Conjunto pela versão gravada no contrato da operadora.
#[must_use]
pub fn conjunto(versao: &str) -> Option<&'static ConjuntoDeRegras> {
    CONJUNTOS.iter().find(|c| c.versao == versao)
}

/// Conjunto usado quando a operadora não tem contrato (só a base, que bloqueia por falta de contrato).
#[must_use]
pub fn conjunto_padrao() -> &'static ConjuntoDeRegras {
    &CONJUNTOS[0]
}

/// O que a auditoria precisa além da guia.
#[derive(Debug, Clone)]
pub struct ContextoAuditoria {
    pub hoje: NaiveDate,
    /// Dias para enviar a guia depois do atendimento (contrato); `None` sem contrato.
    pub prazo_envio_dias: Option<u16>,
    /// Preço vigente por TUSS na tabela da operadora, hoje.
    pub precos_vigentes: BTreeMap<String, Centavos>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pendencia {
    pub regra: CodigoRegra,
    pub efeito: Efeito,
    pub origem: &'static str,
    pub titulo: &'static str,
    /// O caso concreto, sem dado clínico (códigos, datas e valores).
    pub detalhe: String,
    pub o_que_fazer: &'static str,
}

fn data(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

fn ate_20(s: &str) -> bool {
    (1..=20).contains(&s.chars().count())
}

/// Detalhe da regra se ela falha nesta guia; `None` se passa.
fn conferir(regra: CodigoRegra, g: &GuiaFaturavel, ctx: &ContextoAuditoria) -> Option<String> {
    match regra {
        C::OperadoraSemContrato => (g.registro_ans.is_none()
            || g.codigo_prestador.is_none()
            || g.cnes.is_none())
        .then(|| "Sem registro ANS, código do prestador ou CNES para esta operadora.".to_owned()),
        C::GuiaForaDoPadrao => {
            let mut fora = Vec::new();
            if !ate_20(&g.numero_guia_operadora) {
                fora.push("número da guia");
            }
            if !ate_20(&g.senha) {
                fora.push("senha");
            }
            if !ate_20(&g.carteirinha_tiss()) {
                fora.push("carteirinha");
            }
            (!fora.is_empty()).then(|| format!("Fora do padrão: {}.", fora.join(", ")))
        }
        C::SenhaVencida => (g.validade_senha < g.data_execucao).then(|| {
            format!(
                "Senha venceu em {}, exame em {}.",
                data(g.validade_senha),
                data(g.data_execucao)
            )
        }),
        C::ProcedimentoNaoAutorizado => {
            let fora: Vec<String> = g
                .executados
                .iter()
                .filter(|e| {
                    !g.autorizados
                        .iter()
                        .any(|a| a.tuss == e.tuss && a.quantidade >= e.quantidade)
                })
                .map(|e| format!("{} ({}x)", e.tuss, e.quantidade))
                .collect();
            (!fora.is_empty()).then(|| format!("Executado sem autorização: {}.", fora.join(", ")))
        }
        C::SolicitanteIncompleto => (!g.solicitante.completo())
            .then(|| format!("Solicitante {} sem conselho, número, UF ou CBO.", g.solicitante.nome)),
        C::ProcedimentoSemPreco => {
            let sem: Vec<&str> = g
                .executados
                .iter()
                .filter(|e| e.valor_unitario.is_none())
                .map(|e| e.tuss.as_str())
                .collect();
            (!sem.is_empty()).then(|| format!("Sem preço na tabela: {}.", sem.join(", ")))
        }
        C::ValorDiferenteDaTabela => {
            let dif: Vec<String> = g
                .executados
                .iter()
                .filter_map(|e| {
                    let cobrado = e.valor_unitario?;
                    let tabela = *ctx.precos_vigentes.get(&e.tuss)?;
                    (cobrado != tabela).then(|| {
                        format!(
                            "{}: cobrado {}, tabela atual {}",
                            e.tuss,
                            cobrado.reais(),
                            tabela.reais()
                        )
                    })
                })
                .collect();
            (!dif.is_empty()).then(|| format!("{}.", dif.join("; ")))
        }
        C::LaudoNaoAssinado => {
            (!g.laudo_assinado).then(|| "O laudo deste exame ainda não foi assinado.".to_owned())
        }
        C::CidAusente => (!g.cid.as_deref().is_some_and(cid_valido))
            .then(|| "A guia não tem CID-10 válido.".to_owned()),
        C::AssinaturaBeneficiarioAusente => (!g.assinatura_beneficiario)
            .then(|| "Sem registro da assinatura do beneficiário na guia.".to_owned()),
        C::PrazoDeEnvioVencido | C::PrazoDeEnvioProximo => {
            let dias = i64::from(ctx.prazo_envio_dias?);
            let limite = g.data_execucao + chrono::Duration::days(dias);
            let restam = (limite - ctx.hoje).num_days();
            match regra {
                C::PrazoDeEnvioVencido => {
                    (restam < 0).then(|| format!("Envio era até {}.", data(limite)))
                }
                _ => (0..=7)
                    .contains(&restam)
                    .then(|| format!("Envio até {}.", data(limite))),
            }
        }
    }
}

/// Aplica o conjunto de regras à guia. Pendências na ordem do conjunto.
#[must_use]
pub fn auditar(
    g: &GuiaFaturavel,
    ctx: &ContextoAuditoria,
    regras: &ConjuntoDeRegras,
) -> Vec<Pendencia> {
    regras
        .regras
        .iter()
        .filter_map(|r| {
            conferir(r.codigo, g, ctx).map(|detalhe| Pendencia {
                regra: r.codigo,
                efeito: r.efeito,
                origem: r.origem,
                titulo: r.codigo.titulo(),
                detalhe,
                o_que_fazer: r.codigo.o_que_fazer(),
            })
        })
        .collect()
}

/// A guia pode entrar em lote com estas pendências.
#[must_use]
pub fn pronta(pendencias: &[Pendencia]) -> bool {
    !pendencias.iter().any(|p| p.efeito.impede_lote())
}
