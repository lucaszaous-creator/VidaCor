//! Um PDF, um paciente (docs/31 VIS-S1, FC-VIS-03): antes de exportar, as imagens escolhidas
//! precisam ser todas do mesmo paciente e do mesmo estudo.

use crate::{Erro, Estudo};

/// Confere que todos os estudos das imagens escolhidas são o mesmo (paciente e estudo).
/// Devolve o estudo comum. Paciente diferente vence estudo diferente na mensagem, porque é o
/// risco maior (RCP-VIS-01).
pub fn conferir_um_estudo<'a>(
    estudos: impl IntoIterator<Item = &'a Estudo>,
) -> Result<&'a Estudo, Erro> {
    let mut iter = estudos.into_iter();
    let primeiro = iter.next().ok_or(Erro::NadaEscolhido)?;
    let mut estudo_diferente = false;
    for e in iter {
        if e.paciente != primeiro.paciente {
            return Err(Erro::PacientesDiferentes);
        }
        if e.uid != primeiro.uid {
            estudo_diferente = true;
        }
    }
    if estudo_diferente {
        Err(Erro::EstudosDiferentes)
    } else {
        Ok(primeiro)
    }
}
