import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Aplicacao } from './Aplicacao';
import { ponteFalsa } from '../testes/ponteFalsa';

async function abrirExame(ponte = ponteFalsa()) {
  const usuario = userEvent.setup();
  render(<Aplicacao ponte={ponte} />);
  await usuario.click(screen.getByRole('button', { name: 'Abrir pasta' }));
  await screen.findByLabelText(/Paciente em foco: ANA MARIA FICTICIO, nascimento 01\/01\/1980/);
  return { usuario, ponte };
}

// FC-VIS-01: o exame abre com as séries e os dois identificadores na barra.
test('abre o exame com o paciente na barra, as séries e os arquivos que ficaram de fora', async () => {
  await abrirExame();
  const lista = screen.getByRole('navigation', { name: 'Exames e séries' });
  expect(within(lista).getByText('PARTES MOLES')).toBeInTheDocument();
  expect(within(lista).getByText('OSSO')).toBeInTheDocument();
  expect(within(lista).getByText(/mais de um paciente/)).toBeInTheDocument();
  expect(within(lista).getByText('1 arquivo ficou de fora')).toBeInTheDocument();
  expect(screen.getByText(/Imagem 1 de 3 \(nº 3\)/)).toBeInTheDocument();
});

test('trocar de imagem pelo teclado e escolher janela pronta', async () => {
  const { usuario, ponte } = await abrirExame();
  const area = screen.getByLabelText(/Setas trocam a imagem/);
  area.focus();
  await usuario.keyboard('{ArrowDown}');
  expect(await screen.findByText(/Imagem 2 de 3/)).toBeInTheDocument();
  await usuario.click(await screen.findByRole('button', { name: 'Osso' }));
  await waitFor(() => {
    expect(ponte.pixels).toHaveBeenLastCalledWith(1, { estudo: 0, serie: 0, imagem: 1, quadro: 0 }, { centro: 500, largura: 2000 });
  });
});

// FC-VIS-02 (v2) pela tela: todas as séries entram marcadas; confirmar com nome e nascimento.
test('gerar o PDF do exame leva todas as séries e pede confirmação com nome e nascimento', async () => {
  const { usuario, ponte } = await abrirExame();
  const painel = screen.getByRole('complementary', { name: 'Exportar' });
  const pdf = within(painel).getByRole('region', { name: 'PDF do exame' });
  expect(within(pdf).getByRole('checkbox', { name: /Série 2, PARTES MOLES/ })).toBeChecked();
  expect(within(pdf).getByRole('checkbox', { name: /Série 3, OSSO/ })).toBeChecked();
  expect(within(pdf).getByText('O PDF terá 4 imagens em 2 páginas.')).toBeInTheDocument();

  await usuario.click(within(pdf).getByRole('button', { name: 'Gerar PDF do exame' }));
  const dialogo = await screen.findByRole('alertdialog');
  expect(within(dialogo).getByText('ANA MARIA FICTICIO')).toBeInTheDocument();
  expect(within(dialogo).getByText('01/01/1980')).toBeInTheDocument();
  expect(within(dialogo).getByRole('heading')).toHaveTextContent('Gerar o PDF do exame com 4 imagens para este paciente');
  await usuario.click(within(dialogo).getByRole('button', { name: 'Gerar PDF' }));

  expect(ponte.gerarPdf).toHaveBeenCalledWith(1, 0, [
    { serie: 0, janela: null },
    { serie: 1, janela: null },
  ]);
  expect(await screen.findByText(/PDF com 4 imagens salvo/)).toBeInTheDocument();
});

test('a janela escolhida numa série vai para o PDF e a série desmarcada fica fora', async () => {
  const { usuario, ponte } = await abrirExame();
  await usuario.click(await screen.findByRole('button', { name: 'Osso' }));
  const pdf = screen.getByRole('region', { name: 'PDF do exame' });
  await usuario.click(within(pdf).getByRole('checkbox', { name: /Série 3, OSSO/ }));
  expect(within(pdf).getByText('O PDF terá 3 imagens em 1 página.')).toBeInTheDocument();
  await usuario.click(within(pdf).getByRole('button', { name: 'Gerar PDF do exame' }));
  await usuario.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Gerar PDF' }));
  expect(ponte.gerarPdf).toHaveBeenCalledWith(1, 0, [{ serie: 0, janela: { centro: 500, largura: 2000 } }]);

  await usuario.click(within(pdf).getByRole('checkbox', { name: /Série 2, PARTES MOLES/ }));
  expect(within(pdf).getByRole('button', { name: 'Gerar PDF do exame' })).toBeDisabled();
});

// VIS-S1 pela tela: o PDF é sempre do exame mostrado, com o paciente dele na confirmação.
test('o PDF é do exame que está na tela', async () => {
  const { usuario, ponte } = await abrirExame();
  await usuario.click(screen.getByRole('button', { name: /PORTAL/ }));
  await screen.findByLabelText(/Paciente em foco: JOÃO PEDRO FICTICIO, nascimento 15\/06\/1975/);
  await usuario.click(screen.getByRole('button', { name: 'Gerar PDF do exame' }));
  const dialogo = await screen.findByRole('alertdialog');
  expect(within(dialogo).getByText('JOÃO PEDRO FICTICIO')).toBeInTheDocument();
  expect(within(dialogo).queryByText('ANA MARIA FICTICIO')).not.toBeInTheDocument();
  await usuario.click(within(dialogo).getByRole('button', { name: 'Gerar PDF' }));
  expect(ponte.gerarPdf).toHaveBeenCalledWith(1, 1, [{ serie: 0, janela: null }]);
});

test('imagens avulsas marcadas vão para o JPEG', async () => {
  const { usuario, ponte } = await abrirExame();
  await usuario.click(await screen.findByRole('button', { name: 'Marcar para JPEG (M)' }));
  const jpeg = screen.getByRole('region', { name: 'Imagens avulsas em JPEG' });
  expect(within(jpeg).getByText('Janela 35/100')).toBeInTheDocument();
  await usuario.click(within(jpeg).getByRole('button', { name: 'Salvar JPEG' }));
  expect(ponte.exportarJpeg).toHaveBeenCalledWith(1, [{ estudo: 0, serie: 0, imagem: 0, quadro: 0, janela: { centro: 35, largura: 100 } }]);
});

test('erro do programa aparece dizendo o que fazer', async () => {
  const ponte = ponteFalsa({
    gerarPdf: () => Promise.reject(new Error('o exame não tem nome e data de nascimento do paciente; sem os dois identificadores o PDF não é gerado')),
  });
  const { usuario } = await abrirExame(ponte);
  await usuario.click(screen.getByRole('button', { name: 'Gerar PDF do exame' }));
  await usuario.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Gerar PDF' }));
  expect(await screen.findByText('O PDF não foi gerado.')).toBeInTheDocument();
  expect(screen.getByText(/sem os dois identificadores/)).toBeInTheDocument();
});

test('pasta sem imagem de exame avisa e continua na tela inicial', async () => {
  const ponte = ponteFalsa({ abrirPasta: () => Promise.resolve({ abertura: 2, estudos: [], ignorados: [{ nome: 'a.txt', motivo: 'x' }] }) });
  const usuario = userEvent.setup();
  render(<Aplicacao ponte={ponte} />);
  await usuario.click(screen.getByRole('button', { name: 'Abrir pasta' }));
  expect(await screen.findByText('Nenhuma imagem de exame foi encontrada.')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Abrir arquivo .zip ou DICOM' })).toBeInTheDocument();
});
