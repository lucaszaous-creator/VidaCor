//! Eventos para as telas por SSE (REQ-EXA-003). Cada evento declara a unidade e os perfis que podem
//! recebê-lo; a entrega filtra por usuário (SEC-CLI-5: autorização por evento).

use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct EventoTela {
    pub unidade_id: Uuid,
    pub tipo: &'static str,
    pub perfis: &'static [&'static str],
    pub dados: Value,
}

impl EventoTela {
    #[must_use]
    pub fn pode_ver(&self, unidade_id: Uuid, perfis: &[String]) -> bool {
        self.unidade_id == unidade_id && perfis.iter().any(|p| self.perfis.contains(&p.as_str()))
    }
}
