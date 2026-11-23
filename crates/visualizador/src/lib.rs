//! Núcleo do Visualizador VidaCor ([docs/31](../../../docs/31-visualizador-e-exportacao-dicom.md)).
//!
//! Abre exames DICOM de uma pasta, de um `.zip` ou de um arquivo solto ([`abrir`]), agrupa por
//! paciente, estudo e série, ordena os cortes pela posição (VIS-01, VIS-02), decodifica as
//! compressões usadas pelos aparelhos (VIS-03), aplica a janela (VIS-04) e gera JPEG (VIS-11).
//!
//! Este crate não conhece tela nem PDF (G5): a tela (Tauri) e o PDF (Typst) usam o que ele devolve.
//!
//! Proteção do paciente:
//! - nenhum dado do paciente vai para log ou mensagem de erro; o `Debug` de [`Paciente`] esconde
//!   nome, nascimento e ID (VIS-S3);
//! - nome do arquivo JPEG sem dado pessoal ([`nome_arquivo_jpeg`], VIS-11);
//! - limites de quantidade e tamanho antes de ler ou decodificar ([`Limites`], VIS-S6);
//! - pacientes diferentes nunca caem no mesmo estudo, mesmo que o aparelho repita o UID do estudo.

mod cor;
mod erro;
mod exame;
mod fonte;
mod janela;
mod jpeg;
mod render;
mod texto;

pub use erro::Erro;
pub use exame::{abrir, Estudo, Exames, Ignorado, Imagem, MotivoIgnorado, Paciente, Serie};
pub use fonte::{Limites, Origem};
pub use janela::{janelas_prontas, Janela, JanelaPronta};
pub use jpeg::{codificar_jpeg, nome_arquivo_jpeg, QUALIDADE_PADRAO};
pub use render::{decodificar, ImagemRenderizada, Pixels, QuadroCinza, QuadroDecodificado};
pub use texto::data_br;
