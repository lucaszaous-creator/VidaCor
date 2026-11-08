//! Guia SP/SADT faturável: o que o núcleo reúne do atendimento, da guia autorizada, do contrato e da
//! tabela de preços da operadora. Dinheiro sempre em centavos inteiros (sem ponto flutuante).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Valor em centavos de real (nunca negativo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Centavos(pub i64);

impl Centavos {
    /// Formato decimal do TISS (`st_decimal10-2`): ponto e duas casas, ex.: `248.60`.
    #[must_use]
    pub fn tiss(self) -> String {
        format!("{}.{:02}", self.0 / 100, self.0 % 100)
    }

    /// Formato da tela: `R$ 1.248,60`.
    #[must_use]
    pub fn reais(self) -> String {
        let inteiro = (self.0 / 100).to_string();
        let mut com_pontos = String::new();
        for (i, c) in inteiro.chars().enumerate() {
            if i > 0 && (inteiro.len() - i) % 3 == 0 {
                com_pontos.push('.');
            }
            com_pontos.push(c);
        }
        format!("R$ {com_pontos},{:02}", self.0 % 100)
    }

    #[must_use]
    pub fn vezes(self, quantidade: u16) -> Self {
        Self(self.0.saturating_mul(i64::from(quantidade)))
    }
}

/// Profissional solicitante (TISS `ct_contratadoProfissionalDados`). Conselho, número, UF e CBO são
/// obrigatórios no XSD; a guia registrada na recepção só traz o nome, o resto o faturamento completa.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profissional {
    pub nome: String,
    /// Código do conselho no TISS (`dm_conselhoProfissional`), ex.: `06` = CRM.
    pub conselho: Option<String>,
    pub numero: Option<String>,
    /// Código IBGE da UF no TISS (`dm_UF`), ex.: `33` = RJ.
    pub uf: Option<String>,
    /// CBO-S (`dm_CBOS`), ex.: `225125` = médico clínico.
    pub cbos: Option<String>,
}

impl Profissional {
    /// Todos os campos que o XSD exige estão no formato do TISS.
    #[must_use]
    pub fn completo(&self) -> bool {
        let digitos = |v: &Option<String>, min: usize, max: usize| {
            v.as_deref().is_some_and(|s| {
                (min..=max).contains(&s.len()) && s.chars().all(|c| c.is_ascii_digit())
            })
        };
        digitos(&self.conselho, 2, 2)
            && self
                .numero
                .as_deref()
                .is_some_and(|n| (1..=15).contains(&n.chars().count()))
            && digitos(&self.uf, 2, 2)
            && digitos(&self.cbos, 6, 6)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemExecutado {
    pub tuss: String,
    pub descricao: String,
    pub quantidade: u16,
    /// Valor unitário pela tabela da operadora gravado na conta; `None` se o código não está na tabela.
    pub valor_unitario: Option<Centavos>,
}

impl ItemExecutado {
    #[must_use]
    pub fn total(&self) -> Option<Centavos> {
        self.valor_unitario.map(|v| v.vezes(self.quantidade))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemAutorizado {
    pub tuss: String,
    pub quantidade: u16,
}

/// Tudo o que a auditoria confere e o lote leva, de uma guia SP/SADT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiaFaturavel {
    /// Número da guia no prestador (o código da conta, ex.: `FAT-2026-000123`).
    pub numero_guia_prestador: String,
    /// Registro ANS da operadora (6 dígitos); `None` quando a clínica não tem contrato cadastrado.
    pub registro_ans: Option<String>,
    /// Código da clínica na operadora e CNES da unidade, do contrato.
    pub codigo_prestador: Option<String>,
    pub cnes: Option<String>,
    pub numero_guia_operadora: String,
    pub senha: String,
    pub data_autorizacao: NaiveDate,
    pub validade_senha: NaiveDate,
    pub carteirinha: String,
    pub solicitante: Profissional,
    pub data_execucao: NaiveDate,
    pub executados: Vec<ItemExecutado>,
    pub autorizados: Vec<ItemAutorizado>,
    pub laudo_assinado: bool,
    pub cid: Option<String>,
    pub assinatura_beneficiario: bool,
}

impl GuiaFaturavel {
    /// Soma dos itens com preço (os sem preço ficam de fora e já bloqueiam a guia na auditoria).
    #[must_use]
    pub fn valor_total(&self) -> Centavos {
        Centavos(
            self.executados
                .iter()
                .filter_map(ItemExecutado::total)
                .map(|c| c.0)
                .sum(),
        )
    }

    /// Carteirinha como vai no TISS: só letras e dígitos (a recepção digita com espaço e ponto).
    #[must_use]
    pub fn carteirinha_tiss(&self) -> String {
        self.carteirinha
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect()
    }
}

/// CID-10 no formato do TISS: letra, dois dígitos e, opcionalmente, um dígito (sem ponto).
#[must_use]
pub fn cid_valido(cid: &str) -> bool {
    let c: Vec<char> = cid.chars().collect();
    (c.len() == 3 || c.len() == 4)
        && c[0].is_ascii_uppercase()
        && c[1..].iter().all(char::is_ascii_digit)
}
