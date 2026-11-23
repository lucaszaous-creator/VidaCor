//! Registro de exportações (docs/31 VIS-12, docs/21 lastro): uma linha JSON por exportação, só
//! acrescentada, com o hash da linha anterior encadeado. Sem nome, nascimento ou caminho escolhido:
//! quem, quando, estação, estudo (UID), quantas imagens e o SHA-256 do que saiu.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::ErroApp;

const ARQUIVO: &str = "registro-exportacoes.jsonl";
const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone)]
pub struct Lancamento {
    /// "pdf" ou "jpeg".
    pub ato: &'static str,
    pub estudo_uid: String,
    pub imagens: usize,
    /// SHA-256 de cada arquivo gerado.
    pub sha256: Vec<String>,
    /// Versão do modelo do PDF.
    pub modelo: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct Registro {
    caminho: PathBuf,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub(crate) fn sha256(b: &[u8]) -> String {
    hex(&Sha256::digest(b))
}

fn erro(e: impl std::fmt::Display) -> ErroApp {
    ErroApp::Registro(e.to_string())
}

impl Registro {
    /// Abre (ou cria) o registro na pasta e confere que dá para escrever, antes de exportar.
    pub fn abrir(pasta: &Path) -> Result<Registro, ErroApp> {
        fs::create_dir_all(pasta).map_err(erro)?;
        let caminho = pasta.join(ARQUIVO);
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&caminho)
            .map_err(erro)?;
        Ok(Registro { caminho })
    }

    fn linhas(&self) -> Result<Vec<Value>, ErroApp> {
        let texto = fs::read_to_string(&self.caminho).map_err(erro)?;
        texto
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(erro))
            .collect()
    }

    /// Acrescenta um lançamento. `quando` em UTC (RFC 3339).
    pub fn gravar(
        &self,
        l: &Lancamento,
        quando: &str,
        usuario: &str,
        estacao: &str,
    ) -> Result<(), ErroApp> {
        let linhas = self.linhas()?;
        let anterior = linhas
            .last()
            .and_then(|v| v["hash"].as_str())
            .unwrap_or(ZERO)
            .to_owned();
        let conteudo = json!({
            "seq": linhas.len() + 1,
            "quando": quando,
            "usuario": usuario,
            "estacao": estacao,
            "ato": l.ato,
            "estudo_uid": l.estudo_uid,
            "imagens": l.imagens,
            "sha256": l.sha256,
            "modelo": l.modelo,
            "anterior": anterior,
        });
        let hash = sha256(format!("{anterior}{conteudo}").as_bytes());
        let mut linha = conteudo;
        linha["hash"] = Value::String(hash);
        let mut f = OpenOptions::new()
            .append(true)
            .open(&self.caminho)
            .map_err(erro)?;
        writeln!(f, "{linha}").map_err(erro)?;
        f.sync_all().map_err(erro)
    }

    /// Confere a corrente: devolve a quantidade de lançamentos ou o número da primeira linha
    /// alterada, apagada ou fora de ordem.
    pub fn conferir(&self) -> Result<Result<usize, usize>, ErroApp> {
        let linhas = self.linhas()?;
        let mut anterior = ZERO.to_owned();
        for (i, l) in linhas.iter().enumerate() {
            let mut sem_hash = l.clone();
            let hash = sem_hash
                .as_object_mut()
                .and_then(|o| o.remove("hash"))
                .and_then(|h| h.as_str().map(str::to_owned));
            let esperado = sha256(format!("{anterior}{sem_hash}").as_bytes());
            if l["anterior"].as_str() != Some(anterior.as_str())
                || hash.as_deref() != Some(esperado.as_str())
            {
                return Ok(Err(i + 1));
            }
            anterior = esperado;
        }
        Ok(Ok(linhas.len()))
    }
}
