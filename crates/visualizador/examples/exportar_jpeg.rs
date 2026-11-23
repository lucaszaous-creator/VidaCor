//! Converte um exame (pasta, .zip ou arquivo) em JPEG com a janela padrão de cada imagem.
//! Ferramenta de conferência da E1 (docs/31); a tela e o PDF vêm nas etapas E2 e E3.
//!
//! Uso: cargo run -p vidacor-visualizador --example exportar_jpeg -- <exame> <pasta_de_saida>
//!
//! Só mostra contagens e nomes de arquivo sem dado pessoal (VIS-S3).

use std::path::PathBuf;
use std::process::ExitCode;

use vidacor_visualizador::{
    abrir, codificar_jpeg, decodificar, nome_arquivo_jpeg, Limites, QUALIDADE_PADRAO,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (Some(exame), Some(saida)) = (args.get(1), args.get(2)) else {
        eprintln!("uso: exportar_jpeg <exame: pasta, .zip ou arquivo> <pasta de saída>");
        return ExitCode::from(2);
    };
    let saida = PathBuf::from(saida);
    if let Err(e) = std::fs::create_dir_all(&saida) {
        eprintln!("não foi possível criar a pasta de saída: {e}");
        return ExitCode::FAILURE;
    }
    let limites = Limites::default();
    let exames = match abrir(&PathBuf::from(exame), &limites) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("não foi possível abrir o exame: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut gerados = 0;
    let mut falhas = 0;
    for (n, estudo) in exames.estudos.iter().enumerate() {
        println!(
            "estudo {} ({}): {} série(s), {} imagem(ns)",
            n + 1,
            estudo.modalidades().join(", "),
            estudo.series.len(),
            estudo.quantidade_imagens()
        );
        for serie in &estudo.series {
            for img in &serie.imagens {
                for q in 0..img.quadros {
                    let quadro = (img.quadros > 1).then_some(q);
                    let nome = nome_arquivo_jpeg(
                        &estudo.numero_atendimento,
                        serie.numero,
                        img.numero,
                        quadro,
                    );
                    let resultado = decodificar(img, q, &limites)
                        .and_then(|d| codificar_jpeg(&d.renderizar_padrao(), QUALIDADE_PADRAO));
                    match resultado.map(|b| std::fs::write(saida.join(&nome), b)) {
                        Ok(Ok(())) => gerados += 1,
                        Ok(Err(e)) => {
                            falhas += 1;
                            eprintln!("{nome}: não foi possível gravar ({e})");
                        }
                        Err(e) => {
                            falhas += 1;
                            eprintln!("{nome}: {e}");
                        }
                    }
                }
            }
        }
    }
    for i in &exames.ignorados {
        println!("ignorado: {} ({})", i.origem.nome(), i.motivo);
    }
    println!("{gerados} JPEG gerado(s), {falhas} falha(s)");
    if falhas > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
