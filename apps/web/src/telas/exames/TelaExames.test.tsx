import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData, esperarAlertaDoServidor } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

describe('Exames e procedimentos (Exames-Procedimentos.dc.html)', () => {
  it('a recepção consulta preço, situação e histórico, sem mudar nada', async () => {
    estado.sessao = SESSOES.recepcao;
    const usuario = userEvent.setup();
    const { container } = renderizar('/exames');
    expect(await screen.findByText('3 exames ativos, tabela v12 vigente')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Incluir exame' })).not.toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: /Raio X de tórax/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do exame' });
    expect(within(detalhe).getByText('Exame desativado.')).toBeInTheDocument();
    expect(within(detalhe).getByText('Sem preço na tabela vigente')).toBeInTheDocument();
    expect(await within(detalhe).findByText('Desativado, versão 2')).toBeInTheDocument();
    expect(within(detalhe).queryByRole('form', { name: 'Mudar cadastro do exame' })).not.toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('a gerência muda a duração com motivo e a versão vista vai ao servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/exames');
    await usuario.click(
      await screen.findByRole('button', {
        name: /Ecocardiograma transtorácico/,
      }),
    );
    const form = screen.getByRole('form', { name: 'Mudar cadastro do exame' });
    const duracao = within(form).getByRole('textbox', {
      name: 'Duração na agenda (min)',
    });
    await usuario.clear(duracao);
    await usuario.type(duracao, '42');
    await usuario.click(within(form).getByRole('button', { name: 'Salvar mudança' }));
    expect(within(form).getByText('De 5 a 240, de 5 em 5.')).toBeInTheDocument();
    expect(within(form).getByText('Diga o motivo (5 letras ou mais).')).toBeInTheDocument();
    await usuario.clear(duracao);
    await usuario.type(duracao, '40');
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo da mudança' }), 'média real de 40 minutos');
    await usuario.click(within(form).getByRole('button', { name: 'Salvar mudança' }));
    expect(await within(form).findByText('Ecocardiograma transtorácico: versão 2 registrada.')).toBeInTheDocument();
    expect(pedidosDe('mudar-exame')[0]?.corpo).toEqual({
      versao: 1,
      duracao_min: 40,
      modalidade: 'US',
      motivo: 'média real de 40 minutos',
    });
  });

  it('incluir exame com TUSS em uso é recusado pelo servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/exames');
    await usuario.click(await screen.findByRole('button', { name: 'Incluir exame' }));
    const form = screen.getByRole('form', { name: 'Incluir exame' });
    await usuario.type(within(form).getByRole('textbox', { name: 'Nome do exame' }), 'Ecocardiograma de novo');
    await usuario.type(within(form).getByRole('textbox', { name: 'Código TUSS' }), '40901114');
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Duração na agenda' }), '30');
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Modalidade' }), 'US');
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo' }), 'exame novo da Sala 2');
    await usuario.click(within(form).getByRole('button', { name: 'Incluir exame' }));
    await esperarAlertaDoServidor('Este código TUSS já está no catálogo.', 'tuss-em-uso');
    expect(pedidosDe('incluir-exame')[0]?.corpo).toEqual({
      nome: 'Ecocardiograma de novo',
      tuss: '40901114',
      modalidade: 'US',
      duracao_min: 30,
      requer_contraste: false,
      motivo: 'exame novo da Sala 2',
    });
  });

  it('a versão nova da tabela parte dos preços vigentes e começa numa data futura', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/exames');
    await usuario.click(await screen.findByRole('button', { name: 'Nova tabela de preços' }));
    const form = screen.getByRole('form', { name: 'Nova tabela de preços' });
    const eco = within(form).getByRole('textbox', {
      name: 'Ecocardiograma transtorácico (TUSS 40901114)',
    });
    expect(eco).toHaveValue('380,00');
    expect(within(form).queryByRole('textbox', { name: /Raio X de tórax/ })).not.toBeInTheDocument();
    await usuario.click(within(form).getByRole('button', { name: 'Publicar versão' }));
    expect(within(form).getByText('Diga quando começa.')).toBeInTheDocument();
    expect(pedidosDe('publicar-tabela')).toHaveLength(0);
    await digitarData(usuario, 'Começa em', '01012027', form);
    await usuario.type(within(form).getByRole('textbox', { name: 'Nome da versão' }), 'Particular 2027');
    await usuario.clear(eco);
    await usuario.type(eco, '400,00');
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo' }), 'reajuste anual aprovado');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(form).getByRole('button', { name: 'Publicar versão' }));
    expect(await screen.findByText('Tabela v13 publicada; começa em 01/01/2027.')).toBeInTheDocument();
    expect(pedidosDe('publicar-tabela')[0]?.corpo).toEqual({
      vigente_desde: '2027-01-01',
      descricao: 'Particular 2027',
      motivo: 'reajuste anual aprovado',
      precos: [
        { tuss: '40901114', valor_centavos: 40000 },
        { tuss: '40901122', valor_centavos: 18000 },
        { tuss: '41001109', valor_centavos: 95000 },
      ],
    });
  });
});
