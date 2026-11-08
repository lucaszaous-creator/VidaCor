//! Formatos de resposta das rotas de ato simples, publicados no OpenAPI para o cliente gerado
//! (achado da revisão do contrato: respostas sem schema).

use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct EstornoPedido {
    pub id: Uuid,
    pub codigo: String,
    /// pedido.
    pub situacao: String,
}

#[derive(Serialize, ToSchema)]
pub struct EstornoAprovado {
    pub id: Uuid,
    /// aprovado.
    pub situacao: String,
}

#[derive(Serialize, ToSchema)]
pub struct FichaLancada {
    pub id: Uuid,
    pub codigo: String,
    /// contingencia.
    pub origem: String,
    pub contingencia: String,
    pub formulario: String,
    pub ocorrido_em: DateTime<Utc>,
    pub registrado_em_contingencia: bool,
}

#[derive(Serialize, ToSchema)]
pub struct DivergenciaAberta {
    /// divergencia-aberta.
    pub codigo: String,
    /// Código da divergência (DIV-AAAA-NNNNNN).
    pub divergencia: String,
    pub detail: String,
}

#[derive(Serialize, ToSchema)]
pub struct ReciboLancado {
    pub valor_recalculado_centavos: i64,
    pub tabela_versao: i32,
    /// Código da divergência de preço, quando o papel difere do recalculado.
    pub divergencia: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct NumerosSituados {
    pub numeros_situados: u64,
}

#[derive(Serialize, ToSchema)]
pub struct DivergenciaResolvida {
    pub codigo: String,
    /// resolvida.
    pub situacao: String,
}

#[derive(Serialize, ToSchema)]
pub struct EstudoConciliado {
    pub estudo: String,
    pub atendimento: String,
    pub conciliado: bool,
}

#[derive(Serialize, ToSchema)]
pub struct Publicacao {
    pub publicado: bool,
    /// Falso quando a versão já estava publicada (repetir não duplica).
    pub nova_publicacao: bool,
}

#[derive(Serialize, ToSchema)]
pub struct AcessoEmergencia {
    pub codigo: String,
    pub valido_ate: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct WebhookAceito {
    /// recebido ou ja_processado.
    pub situacao: String,
}

/// Serializa a resposta tipada (mesmo formato do schema publicado).
///
/// # Errors
/// Falha de serialização (não esperada).
pub fn valor<T: Serialize>(t: &T) -> Result<serde_json::Value, crate::erro::ErroApi> {
    serde_json::to_value(t).map_err(crate::erro::ErroApi::interno)
}
