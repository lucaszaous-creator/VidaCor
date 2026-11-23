//! Visualizador VidaCor (docs/31): programa Tauri que abre exames DICOM e exporta PDF e JPEG.
//!
//! Divisão (G1, G5): a tela (React) só mostra e pede; tudo o que decide fica aqui no Rust: qual
//! arquivo abrir (o diálogo é aberto pelo Rust, a tela nunca manda caminho), os dados do paciente
//! impressos no PDF (lidos do exame, nunca da tela), a regra "um PDF, um paciente" (VIS-S1), os nomes
//! dos arquivos e o registro de cada exportação (VIS-12).

mod comandos;
mod dto;
mod erro;
mod estado;
mod exportar;
mod registro;

pub use dto::ItemDto;
pub use erro::ErroApp;
pub use exportar::{
    jpegs, legenda, montar_pdf, nome_livre, nome_pdf, resolver, Grade, ItemResolvido, CLINICA,
};
pub use registro::{Lancamento, Registro};

/// Inicia o programa.
pub fn executar() {
    let resultado = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(estado::Estado::default())
        .invoke_handler(tauri::generate_handler![
            comandos::abrir_pasta,
            comandos::abrir_arquivo,
            comandos::quadro,
            comandos::pixels,
            comandos::acoes_da_imagem,
            comandos::exportar_jpeg,
            comandos::gerar_pdf,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = resultado {
        // Sem dado do paciente aqui: só a falha do próprio programa.
        eprintln!("o Visualizador VidaCor não conseguiu iniciar: {e}");
        std::process::exit(1);
    }
}
