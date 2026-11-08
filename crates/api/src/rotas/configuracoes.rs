//! Configurações da unidade (Configuracoes.dc.html): situação de cada integração externa, sem valor
//! nenhum de configuração (só o nome da variável e se está preenchida) e o que está parado no outbox
//! esperando por ela (DEC-2026-11-08-78). TI e gerência (docs/18: a TI administra integrações).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::integracoes::{
    preenchida, situacao, tipos_do_outbox, IntegracaoVista, VariavelVista, INTEGRACOES,
};
use crate::sessao::Usuario;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Situação das integrações (e-mail, WhatsApp, NFS-e, PIX, assinatura, visualizador).
#[utoipa::path(get, operation_id = "situacao_das_integracoes", path = "/v1/configuracoes/integracoes", tag = "configuracoes",
    responses((status = 200, body = Vec<IntegracaoVista>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn integracoes(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::AparelhoAdministrar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let mut lista = Vec::new();
    for d in INTEGRACOES {
        let ativa = match d.id {
            "email" => estado.correio.is_some(),
            "whatsapp" => estado.mensageiro.is_some(),
            "nfse" => estado.prefeitura.is_some(),
            "pix" => estado.psp.is_some(),
            "assinatura" => estado.assinador.is_some(),
            "visualizador" => estado.config.visualizador_url.is_some(),
            _ => false,
        };
        let tipos: Vec<String> = tipos_do_outbox(d.id)
            .iter()
            .map(|t| (*t).to_owned())
            .collect();
        let pendentes = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM integracao.outbox WHERE unidade_id = $1 AND concluido_em IS NULL AND tipo = ANY($2)"#,
            usuario.unidade_id,
            &tipos
        )
        .fetch_one(&mut *tx)
        .await?;
        let (sit, falta) = situacao(d, ativa);
        lista.push(IntegracaoVista {
            id: d.id.to_owned(),
            nome: d.nome.to_owned(),
            para_que: d.para_que.to_owned(),
            situacao: sit,
            o_que_falta: falta,
            variaveis: d
                .variaveis
                .iter()
                .map(|x| VariavelVista {
                    nome: x.nome.to_owned(),
                    descricao: x.descricao.to_owned(),
                    obrigatoria: x.obrigatoria,
                    segredo: x.segredo,
                    preenchida: preenchida(x.nome),
                })
                .collect(),
            pendentes_no_outbox: pendentes,
        });
    }
    tx.commit().await?;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(lista)).into_response())
}
