// Primeira tela: abrir um exame. O diálogo de arquivo é do Rust; a tela só pede.
import { Botao } from '@vidacor/design-system';
import logo from '../../../../design/assets/marca/logo.png';

export interface PropsDaTelaInicial {
  aoAbrirPasta: () => void;
  aoAbrirArquivo: () => void;
  ocupado: boolean;
}

export function TelaInicial({ aoAbrirPasta, aoAbrirArquivo, ocupado }: PropsDaTelaInicial) {
  return (
    <main className="vz-inicio">
      <div className="vz-inicio-cartao">
        <img src={logo} alt="VidaCor Imagem" width={170} />
        <h1>Visualizador de exames</h1>
        <p>Abra um exame para ver as imagens e gerar o PDF ou os JPEG do paciente.</p>
        <div className="row" style={{ gap: 10, justifyContent: 'center' }}>
          <Botao variante="principal" ocupado={ocupado} onPress={aoAbrirPasta}>
            Abrir pasta
          </Botao>
          <Botao ocupado={ocupado} onPress={aoAbrirArquivo}>
            Abrir arquivo .zip ou DICOM
          </Botao>
        </div>
        <p className="t13 mute">Pasta inclui CD e DVD de exame. Arquivos que não forem imagem de exame ficam de fora, com o motivo.</p>
      </div>
    </main>
  );
}
