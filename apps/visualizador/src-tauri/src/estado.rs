//! Estado do programa: o exame aberto e os últimos quadros decodificados (para o ajuste de janela
//! com o mouse não reler o arquivo).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use vidacor_visualizador::{Exames, Imagem, QuadroDecodificado};

use crate::ErroApp;

/// Quadros guardados na memória (navegação para frente e para trás sem reler).
const CACHE: usize = 24;

pub struct Aberto {
    pub id: u64,
    pub exames: Exames,
}

/// (abertura, estudo, série, imagem, quadro)
pub type ChaveQuadro = (u64, usize, usize, usize, u32);

#[derive(Default)]
pub struct Estado {
    proximo: AtomicU64,
    aberto: Mutex<Option<Aberto>>,
    cache: Mutex<Vec<(ChaveQuadro, Arc<QuadroDecodificado>)>>,
}

fn travar<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // Um comando que falhou no meio não deixa o programa inutilizável.
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Estado {
    /// Troca o exame aberto e devolve o número da nova abertura.
    pub fn abrir(&self, exames: Exames) -> u64 {
        let id = self.proximo.fetch_add(1, Ordering::Relaxed) + 1;
        *travar(&self.aberto) = Some(Aberto { id, exames });
        travar(&self.cache).clear();
        id
    }

    /// Executa `f` com o exame aberto, se for o da abertura pedida.
    pub fn com_exame<R>(
        &self,
        abertura: u64,
        f: impl FnOnce(&Exames) -> Result<R, ErroApp>,
    ) -> Result<R, ErroApp> {
        let guarda = travar(&self.aberto);
        match guarda.as_ref() {
            Some(a) if a.id == abertura => f(&a.exames),
            _ => Err(ErroApp::ExameTrocado),
        }
    }

    /// Imagem e modalidade da série, copiadas para decodificar fora da trava.
    pub fn imagem(&self, chave: ChaveQuadro) -> Result<(Imagem, String), ErroApp> {
        let (abertura, e, s, i, q) = chave;
        self.com_exame(abertura, |ex| {
            let serie = ex
                .estudos
                .get(e)
                .and_then(|x| x.series.get(s))
                .ok_or(ErroApp::ItemInexistente)?;
            let img = serie.imagens.get(i).ok_or(ErroApp::ItemInexistente)?;
            if q >= img.quadros {
                return Err(ErroApp::ItemInexistente);
            }
            Ok((img.clone(), serie.modalidade.clone()))
        })
    }

    pub fn do_cache(&self, chave: ChaveQuadro) -> Option<Arc<QuadroDecodificado>> {
        let mut c = travar(&self.cache);
        let pos = c.iter().position(|(k, _)| *k == chave)?;
        let item = c.remove(pos);
        let q = Arc::clone(&item.1);
        c.push(item);
        Some(q)
    }

    pub fn guardar(&self, chave: ChaveQuadro, q: Arc<QuadroDecodificado>) {
        let mut c = travar(&self.cache);
        c.retain(|(k, _)| *k != chave);
        if c.len() >= CACHE {
            c.remove(0);
        }
        c.push((chave, q));
    }
}
