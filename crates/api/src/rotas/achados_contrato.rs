//! Contrato HTTP do laudo estruturado (docs/22 E2): os descritores que a tela envia e o achado que
//! o núcleo devolve. Os enums espelham os do domínio (`vidacor_dominio_laudo`) com os mesmos
//! códigos; a conversão é exaustiva, então um descritor novo no domínio quebra a compilação aqui
//! até o contrato ser atualizado. O cálculo fica todo no domínio (G1, G5).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_dominio_laudo::birads::{
    Achado as BAchado, Categoria as BCategoria, Forma as BForma, Lado as BLado, Margem as BMargem,
    Massa as BMassa, Orientacao as BOrientacao, PadraoEcogenico as BPadraoEcogenico,
    Posterior as BPosterior,
};
use vidacor_dominio_laudo::tirads::{
    Composicao as TComposicao, Ecogenicidade as TEcogenicidade, FocoEcogenico as TFoco,
    Forma as TForma, Margem as TMargem, Nodulo as TNodulo,
};

/// Enum do contrato com os mesmos valores do domínio, nos dois sentidos.
macro_rules! espelho {
    ($(#[$m:meta])* $nome:ident <=> $dominio:ident { $($v:ident),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
        #[serde(rename_all = "snake_case")]
        pub enum $nome { $($v),+ }
        impl From<$nome> for $dominio {
            fn from(v: $nome) -> Self { match v { $($nome::$v => $dominio::$v),+ } }
        }
        impl From<$dominio> for $nome {
            fn from(v: $dominio) -> Self { match v { $($dominio::$v => $nome::$v),+ } }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SistemaClassificacao {
    /// ACR TI-RADS 2017 (nódulo de tireoide).
    Tirads,
    /// BI-RADS 5ª edição, ultrassom de mama.
    Birads,
}

impl SistemaClassificacao {
    #[must_use]
    pub const fn codigo(self) -> &'static str {
        match self {
            Self::Tirads => "tirads",
            Self::Birads => "birads",
        }
    }
}

espelho!(ComposicaoTirads <=> TComposicao { CisticaOuQuaseCistica, Espongiforme, MistaCisticaESolida, SolidaOuQuaseSolida, IndeterminadaPorCalcificacao });
espelho!(EcogenicidadeTirads <=> TEcogenicidade { Anecoica, HiperOuIsoecoica, Hipoecoica, MuitoHipoecoica, Indeterminada });
espelho!(FormaTirads <=> TForma { MaisLargoQueAlto, MaisAltoQueLargo });
espelho!(MargemTirads <=> TMargem { Lisa, MalDefinida, LobuladaOuIrregular, ExtensaoExtratireoidiana, Indeterminada });
espelho!(FocoTirads <=> TFoco { NenhumOuCaudaDeCometa, Macrocalcificacoes, CalcificacoesPerifericas, FocosPuntiformes });
espelho!(LadoBirads <=> BLado { Direita, Esquerda });
espelho!(FormaBirads <=> BForma { Oval, Redonda, Irregular });
espelho!(OrientacaoBirads <=> BOrientacao { Paralela, NaoParalela });
espelho!(MargemBirads <=> BMargem { Circunscrita, Indistinta, Angular, Microlobulada, Espiculada });
espelho!(PadraoEcogenicoBirads <=> BPadraoEcogenico { Anecoico, Hiperecoico, ComplexoCisticoESolido, Hipoecoico, Isoecoico, Heterogeneo });
espelho!(PosteriorBirads <=> BPosterior { Nenhum, Reforco, Sombra, Combinado });

/// Categoria BI-RADS atribuída pelo médico (códigos do atlas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum CategoriaBirads {
    #[serde(rename = "0")]
    Zero,
    #[serde(rename = "1")]
    Um,
    #[serde(rename = "2")]
    Dois,
    #[serde(rename = "3")]
    Tres,
    #[serde(rename = "4A")]
    QuatroA,
    #[serde(rename = "4B")]
    QuatroB,
    #[serde(rename = "4C")]
    QuatroC,
    #[serde(rename = "5")]
    Cinco,
    #[serde(rename = "6")]
    Seis,
}

impl From<CategoriaBirads> for BCategoria {
    fn from(v: CategoriaBirads) -> Self {
        use CategoriaBirads as C;
        match v {
            C::Zero => Self::Zero,
            C::Um => Self::Um,
            C::Dois => Self::Dois,
            C::Tres => Self::Tres,
            C::QuatroA => Self::QuatroA,
            C::QuatroB => Self::QuatroB,
            C::QuatroC => Self::QuatroC,
            C::Cinco => Self::Cinco,
            C::Seis => Self::Seis,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DescritoresTirads {
    pub composicao: ComposicaoTirads,
    pub ecogenicidade: EcogenicidadeTirads,
    pub forma: FormaTirads,
    pub margem: MargemTirads,
    /// Pode ter mais de um (os pontos somam); vazio vale "nenhum".
    pub focos: Vec<FocoTirads>,
}

impl From<&DescritoresTirads> for TNodulo {
    fn from(d: &DescritoresTirads) -> Self {
        Self {
            composicao: d.composicao.into(),
            ecogenicidade: d.ecogenicidade.into(),
            forma: d.forma.into(),
            margem: d.margem.into(),
            focos: d.focos.iter().map(|&f| f.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MassaBirads {
    pub forma: FormaBirads,
    pub orientacao: OrientacaoBirads,
    pub margens: Vec<MargemBirads>,
    pub padrao_ecogenico: PadraoEcogenicoBirads,
    pub posterior: PosteriorBirads,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DescritoresBirads {
    pub lado: LadoBirads,
    /// Categoria final escolhida pelo médico; o servidor confere a coerência com a massa.
    pub categoria_atribuida: CategoriaBirads,
    pub biopsia_maligna_confirmada: bool,
    /// Ausente em BI-RADS 1 (negativo) e, se o médico quiser, em 0.
    pub massa: Option<MassaBirads>,
}

impl From<&DescritoresBirads> for BAchado {
    fn from(d: &DescritoresBirads) -> Self {
        Self {
            lado: d.lado.into(),
            categoria_atribuida: d.categoria_atribuida.into(),
            biopsia_maligna_confirmada: d.biopsia_maligna_confirmada,
            massa: d.massa.as_ref().map(|m| BMassa {
                forma: m.forma.into(),
                orientacao: m.orientacao.into(),
                margens: m.margens.iter().map(|&g| g.into()).collect(),
                padrao_ecogenico: m.padrao_ecogenico.into(),
                posterior: m.posterior.into(),
            }),
        }
    }
}

/// Novo achado. Só os descritores do sistema escolhido são lidos; pontos, categoria e conduta
/// enviados pela tela são ignorados (o servidor calcula, G1).
#[derive(Debug, Deserialize, ToSchema)]
pub struct NovoAchado {
    pub sistema: SistemaClassificacao,
    /// Ex.: "Lobo direito, terço médio" ou "Quadrante superolateral, 2 horas" (até 120 caracteres).
    pub localizacao: String,
    /// Maior diâmetro em milímetros (uma casa decimal). Obrigatório no TI-RADS e quando há massa.
    pub maior_diametro_mm: Option<f64>,
    pub tirads: Option<DescritoresTirads>,
    pub birads: Option<DescritoresBirads>,
}

/// Achado estruturado com o resultado calculado pelo servidor e a versão da regra aplicada.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Achado {
    pub id: Uuid,
    pub codigo: String,
    /// Ordem no laudo (N1, N2...).
    pub numero: i32,
    pub rotulo: String,
    /// Versão do laudo a que o achado pertence.
    pub versao_numero: i32,
    pub sistema: SistemaClassificacao,
    pub localizacao: String,
    pub maior_diametro_mm: Option<f64>,
    pub tirads: Option<DescritoresTirads>,
    pub birads: Option<DescritoresBirads>,
    /// Pontos do TI-RADS (ausente no BI-RADS).
    pub pontos: Option<i32>,
    /// TR1 a TR5 ou 0 a 6 (com 4A, 4B, 4C).
    pub categoria: String,
    pub categoria_nome: String,
    pub conduta: String,
    /// Ex.: TIRADS-ACR-2017.1. Gravada no achado; mudança de versão não altera achados antigos.
    pub regra_versao: String,
    /// Frase sugerida para o texto do laudo (o médico revisa e edita).
    pub texto_achado: String,
    pub texto_conclusao: String,
    pub criado_em: DateTime<Utc>,
}

/// Regra vigente de um sistema de classificação.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegraVigente {
    pub sistema: SistemaClassificacao,
    pub versao: String,
}

/// Achados da versão em edição do laudo (ou da vigente, se assinado).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AchadosDoLaudo {
    pub laudo_id: Uuid,
    pub versao_numero: i32,
    /// Verdadeiro só para o médico atribuído com versão em rascunho.
    pub editavel: bool,
    pub achados: Vec<Achado>,
    /// Frases dos achados, uma por linha, para o médico revisar e levar ao texto do laudo.
    pub texto_achados: String,
    pub texto_conclusao: String,
    pub regras_vigentes: Vec<RegraVigente>,
}
