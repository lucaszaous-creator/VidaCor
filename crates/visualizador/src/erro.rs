/// Erros do visualizador. As mensagens dizem o que aconteceu sem dado do paciente nem caminho de
/// arquivo (o nome da pasta pode ser o nome do paciente): quem chama sabe qual origem pediu.
#[derive(Debug, thiserror::Error)]
pub enum Erro {
    #[error("o arquivo ou a pasta não foi encontrado")]
    NaoEncontrado,
    #[error("não foi possível ler o arquivo ({0})")]
    Leitura(#[source] std::io::Error),
    #[error("o arquivo .zip está danificado ou não é um .zip")]
    ZipInvalido,
    #[error("o limite de {limite} foi ultrapassado; abra o exame em partes")]
    LimiteExcedido { limite: &'static str },
    #[error("o arquivo não é um exame DICOM")]
    NaoDicom,
    #[error("o arquivo DICOM está danificado ou incompleto")]
    Danificado,
    #[error("o arquivo não tem imagem")]
    SemImagem,
    #[error("a imagem usa uma compressão ou um formato que o visualizador ainda não lê")]
    FormatoNaoSuportado,
    #[error("o quadro pedido não existe nesta imagem")]
    QuadroInexistente,
    #[error("a imagem é grande demais para gerar JPEG")]
    ImagemGrandeDemais,
    #[error("não foi possível gerar o JPEG")]
    Jpeg,
}
