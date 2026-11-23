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

// FC-VIS-02 pela tela: confirmar com nome e nascimento antes de gerar o PDF.
test('marcar e gerar o PDF pede confirmação com o nome e o nascimento', async () => {
  const { usuario, ponte } = await abrirExame();
  await usuario.click(await screen.findByRole('button', { name: 'Marcar para exportar (M)' }));
  const painel = screen.getByRole('complementary', { name: 'Exportar' });
  expect(within(painel).getByText(/1 de no máximo 600/)).toBeInTheDocument();
  expect(within(painel).getByText('Janela 35/100')).toBeInTheDocument();

  await usuario.click(within(painel).getByRole('button', { name: 'Gerar PDF do paciente' }));
  const dialogo = await screen.findByRole('alertdialog');
  expect(within(dialogo).getByText('ANA MARIA FICTICIO')).toBeInTheDocument();
  expect(within(dialogo).getByText('01/01/1980')).toBeInTheDocument();
  await usuario.click(within(dialogo).getByRole('button', { name: 'Gerar PDF' }));

  expect(ponte.gerarPdf).toHaveBeenCalledWith(1, [{ estudo: 0, serie: 0, imagem: 0, quadro: 0, janela: { centro: 35, largura: 100 } }], 6);
  expect(await screen.findByText(/PDF com 1 imagem salvo/)).toBeInTheDocument();
});

// FC-VIS-03 pela tela: exames diferentes marcados apagam o botão do PDF com o motivo.
test('imagens de dois exames não deixam gerar o PDF', async () => {
  const { usuario, ponte } = await abrirExame();
  await usuario.click(await screen.findByRole('button', { name: 'Marcar para exportar (M)' }));
  await usuario.click(screen.getByRole('button', { name: /PORTAL/ }));
  await usuario.click(await screen.findByRole('button', { name: 'Marcar para exportar (M)' }));
  const painel = screen.getByRole('complementary', { name: 'Exportar' });
  expect(within(painel).getByRole('alert')).toHaveTextContent('exames diferentes');
  expect(within(painel).getByRole('button', { name: 'Gerar PDF do paciente' })).toBeDisabled();
  expect(ponte.gerarPdf).not.toHaveBeenCalled();
});

test('erro do programa aparece dizendo o que fazer', async () => {
  const ponte = ponteFalsa({
    gerarPdf: () => Promise.reject(new Error('as imagens escolhidas são de pacientes diferentes; um PDF leva imagens de um só paciente')),
  });
  const { usuario } = await abrirExame(ponte);
  await usuario.click(await screen.findByRole('button', { name: 'Marcar para exportar (M)' }));
  await usuario.click(screen.getByRole('button', { name: 'Gerar PDF do paciente' }));
  await usuario.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Gerar PDF' }));
  expect(await screen.findByText('O PDF não foi gerado.')).toBeInTheDocument();
  expect(screen.getByText(/um PDF leva imagens de um só paciente/)).toBeInTheDocument();
});

test('pasta sem imagem de exame avisa e continua na tela inicial', async () => {
  const ponte = ponteFalsa({ abrirPasta: () => Promise.resolve({ abertura: 2, estudos: [], ignorados: [{ nome: 'a.txt', motivo: 'x' }] }) });
  const usuario = userEvent.setup();
  render(<Aplicacao ponte={ponte} />);
  await usuario.click(screen.getByRole('button', { name: 'Abrir pasta' }));
  expect(await screen.findByText('Nenhuma imagem de exame foi encontrada.')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Abrir arquivo .zip ou DICOM' })).toBeInTheDocument();
});
