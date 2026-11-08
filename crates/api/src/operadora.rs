//! Porta do webservice TISS da operadora (provedor externo: único ponto simulado nos testes, G4).
//! O lote é enviado com a chave de idempotência gerada uma vez na criação do lote e reaproveitada
//! nas retentativas (G3). Sem operadora configurada, o envio fica pendente no outbox, visível na
//! tela do lote (R7). O pedido de autorização (docs/22 E4) vai com a chave do pedido e devolve a
//! resposta bruta da operadora, que o núcleo grava como recebida antes de interpretar.

use crate::assinatura::Futuro;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use uuid::Uuid;

/// Protocolo de recebimento devolvido pela operadora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protocolo {
    pub numero: String,
}

pub trait Operadora: Send + Sync {
    /// Envia o arquivo do lote (`tissLoteGuias`) à operadora de registro ANS `registro_ans`.
    fn enviar_lote<'a>(
        &'a self,
        chave: Uuid,
        registro_ans: &'a str,
        arquivo: &'a [u8],
    ) -> Futuro<'a, anyhow::Result<Protocolo>>;

    /// Envia a solicitação de procedimento (`tissSolicitacaoProcedimento`) e devolve a resposta bruta.
    fn solicitar_autorizacao<'a>(
        &'a self,
        chave: Uuid,
        registro_ans: &'a str,
        numero_guia_prestador: &'a str,
        arquivo: &'a [u8],
    ) -> Futuro<'a, anyhow::Result<Vec<u8>>>;

    /// Consulta a situação de um pedido em análise (`tissSolicitacaoStatusAutorizacao`); devolve a
    /// resposta bruta.
    fn consultar_autorizacao<'a>(
        &'a self,
        registro_ans: &'a str,
        numero_guia_prestador: &'a str,
    ) -> Futuro<'a, anyhow::Result<Vec<u8>>>;
}

/// Próxima resposta da operadora de desenvolvimento (programada pelos testes).
#[derive(Debug, Clone)]
pub struct Roteiro {
    pub status: vidacor_dominio_faturamento::autorizacao::StatusAutorizacao,
    /// TUSS, quantidade pedida e autorizada.
    pub procedimentos: Vec<(String, u16, u16)>,
    pub motivos: Vec<String>,
}

/// Operadora de DESENVOLVIMENTO em memória: idempotente pela chave, sem envio de verdade.
#[derive(Default)]
pub struct OperadoraDesenvolvimento {
    protocolos: Mutex<HashMap<Uuid, Protocolo>>,
    /// Quantas vezes o envio foi chamado (inclusive repetições com a mesma chave).
    pub chamadas: std::sync::atomic::AtomicUsize,
    roteiro: Mutex<VecDeque<Roteiro>>,
    autorizacoes: Mutex<HashMap<Uuid, Vec<u8>>>,
    /// Consultas de pedido em análise recebidas.
    pub consultas: std::sync::atomic::AtomicUsize,
}

impl Operadora for OperadoraDesenvolvimento {
    fn enviar_lote<'a>(
        &'a self,
        chave: Uuid,
        _registro_ans: &'a str,
        arquivo: &'a [u8],
    ) -> Futuro<'a, anyhow::Result<Protocolo>> {
        Box::pin(async move {
            self.chamadas
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            anyhow::ensure!(!arquivo.is_empty(), "arquivo vazio");
            let mut mapa = self
                .protocolos
                .lock()
                .map_err(|_| anyhow::anyhow!("operadora indisponível"))?;
            let n = mapa.len() + 1;
            Ok(mapa
                .entry(chave)
                .or_insert_with(|| Protocolo {
                    numero: format!("DEV-{n:08}"),
                })
                .clone())
        })
    }

    fn solicitar_autorizacao<'a>(
        &'a self,
        chave: Uuid,
        registro_ans: &'a str,
        numero_guia_prestador: &'a str,
        arquivo: &'a [u8],
    ) -> Futuro<'a, anyhow::Result<Vec<u8>>> {
        Box::pin(async move {
            anyhow::ensure!(!arquivo.is_empty(), "solicitação vazia");
            if let Some(r) = self
                .autorizacoes
                .lock()
                .map_err(|_| anyhow::anyhow!("operadora indisponível"))?
                .get(&chave)
            {
                return Ok(r.clone());
            }
            let r = self.proxima(registro_ans, numero_guia_prestador)?;
            self.autorizacoes
                .lock()
                .map_err(|_| anyhow::anyhow!("operadora indisponível"))?
                .entry(chave)
                .or_insert(r.clone());
            Ok(r)
        })
    }

    fn consultar_autorizacao<'a>(
        &'a self,
        registro_ans: &'a str,
        numero_guia_prestador: &'a str,
    ) -> Futuro<'a, anyhow::Result<Vec<u8>>> {
        Box::pin(async move {
            self.consultas
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.proxima(registro_ans, numero_guia_prestador)
        })
    }
}

impl OperadoraDesenvolvimento {
    /// Programa as próximas respostas (uma por solicitação ou consulta, na ordem). Sem roteiro, a
    /// operadora responde "em análise".
    pub fn programar(&self, respostas: Vec<Roteiro>) {
        if let Ok(mut r) = self.roteiro.lock() {
            r.extend(respostas);
        }
    }

    /// Pedidos distintos recebidos (um por chave).
    #[must_use]
    pub fn solicitacoes_recebidas(&self) -> usize {
        self.autorizacoes.lock().map(|m| m.len()).unwrap_or(0)
    }

    fn proxima(&self, registro_ans: &str, numero: &str) -> anyhow::Result<Vec<u8>> {
        use vidacor_dominio_faturamento::autorizacao::{
            resposta_simulada, RespostaSimulada, StatusAutorizacao,
        };
        let r = self
            .roteiro
            .lock()
            .map_err(|_| anyhow::anyhow!("operadora indisponível"))?
            .pop_front()
            .unwrap_or(Roteiro {
                status: StatusAutorizacao::EmAnalise,
                procedimentos: Vec::new(),
                motivos: Vec::new(),
            });
        let agora = chrono::Utc::now().with_timezone(&chrono_tz::America::Sao_Paulo);
        let hoje = agora.date_naive();
        let com_senha = matches!(
            r.status,
            StatusAutorizacao::Autorizada | StatusAutorizacao::Parcial
        );
        let senha = format!(
            "DEV-{}",
            numero
                .chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
        );
        let procs: Vec<(&str, u16, u16)> = r
            .procedimentos
            .iter()
            .map(|(t, p, a)| (t.as_str(), *p, *a))
            .collect();
        let motivos: Vec<&str> = r.motivos.iter().map(String::as_str).collect();
        let numero_operadora = format!("OP{}", &senha[4..]);
        Ok(resposta_simulada(&RespostaSimulada {
            numero_guia_prestador: numero,
            registro_ans,
            codigo_prestador: "0000000",
            carteirinha: "0",
            status: r.status,
            numero_guia_operadora: com_senha.then_some(numero_operadora.as_str()),
            senha: com_senha.then_some(senha.as_str()),
            data_autorizacao: com_senha.then_some(hoje),
            validade: com_senha.then(|| hoje + chrono::Duration::days(30)),
            procedimentos: &procs,
            motivos: &motivos,
            // Sem hora no carimbo: a mesma resposta no mesmo dia sai com os mesmos bytes (como uma
            // operadora que repete o documento), o que permite provar que a repetição não duplica.
            quando: hoje.and_hms_opt(0, 0, 0).unwrap_or_default(),
        }))
    }

    /// Protocolos distintos emitidos (um por chave).
    #[must_use]
    pub fn protocolos_emitidos(&self) -> usize {
        self.protocolos.lock().map(|m| m.len()).unwrap_or(0)
    }
}
