/// Erros do programa. A mensagem diz o que aconteceu e o que fazer (R8), sem dado do paciente.
#[derive(Debug, thiserror::Error)]
pub enum ErroApp {
    #[error("{0}")]
    Visualizador(#[from] vidacor_visualizador::Erro),
    #[error("não foi possível gerar o PDF: {0}")]
    Documento(String),
    #[error("o exame foi fechado ou trocado; abra o exame de novo")]
    ExameTrocado,
    #[error("a imagem pedida não existe neste exame; abra o exame de novo")]
    ItemInexistente,
    #[error("no máximo {0} imagens por exportação; escolha menos imagens")]
    MuitasImagens(usize),
    #[error("o exame não tem nome e data de nascimento do paciente; sem os dois identificadores o PDF não é gerado")]
    SemIdentificadores,
    #[error("não foi possível gravar o arquivo na pasta escolhida ({0}); escolha outra pasta")]
    Gravacao(String),
    #[error("só {salvas} de {total} imagens foram salvas; a pasta recusou as demais ({motivo}). As salvas ficaram registradas; escolha outra pasta para as que faltam")]
    GravacaoParcial {
        salvas: usize,
        total: usize,
        motivo: String,
    },
    #[error("não foi possível gravar o registro de exportações ({0}); nada foi exportado. Chame o suporte de TI")]
    Registro(String),
    #[error(
        "o arquivo foi salvo, mas o registro da exportação falhou ({0}); avise o suporte de TI"
    )]
    RegistroAposSalvar(String),
}

impl serde::Serialize for ErroApp {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
