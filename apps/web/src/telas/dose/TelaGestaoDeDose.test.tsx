import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.medicaTc;
});

describe('Gestão de dose (Gestao-Dose.dc.html)', () => {
  it('o médico vê a dose de cada exame comparada ao NRD, com nome e nascimento', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/gestao-de-dose');
    const tabela = await screen.findByRole('table', { name: 'Exames com dose' });
    const linhas = within(tabela).getAllByRole('row');
    expect(within(linhas[1] as HTMLElement).getByText('Antônio Carlos Ribeiro')).toBeInTheDocument();
    expect(within(linhas[1] as HTMLElement).getByText('nascimento 15/06/1954')).toBeInTheDocument();
    expect(within(linhas[1] as HTMLElement).getByText('Acima do NRD')).toBeInTheDocument();
    expect(within(linhas[1] as HTMLElement).getByText('NRD 12 mGy, 18% acima')).toBeInTheDocument();
    expect(within(linhas[2] as HTMLElement).getByText('Dentro do NRD')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(linhas[1] as HTMLElement).getByText('TC de tórax'));
    expect(await screen.findByText('7,28 mSv', { selector: 'b' })).toBeInTheDocument();
    expect(screen.getByText(/não é dosimetria individual/)).toBeInTheDocument();
  });

  it('justifica a dose acima do NRD com motivo; "Outro" exige descrição', async () => {
    const usuario = userEvent.setup();
    renderizar('/gestao-de-dose');
    await usuario.click(await screen.findByRole('button', { name: 'Justificar' }));
    const form = screen.getByRole('form', { name: 'Justificativa de dose' });
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Motivo' }), 'Outro');
    await usuario.click(within(form).getByRole('button', { name: 'Registrar justificativa' }));
    expect(within(form).getByText('Em "Outro", descreva o motivo (5 letras ou mais).')).toBeInTheDocument();
    expect(pedidosDe('justificativa')).toHaveLength(0);
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Motivo' }), 'Fase extra pedida pelo médico');
    await usuario.click(within(form).getByRole('button', { name: 'Registrar justificativa' }));
    expect(await screen.findByText('Justificativa registrada.')).toBeInTheDocument();
    const p = pedidosDe('justificativa').at(-1);
    expect(p?.corpo).toEqual({ motivo: 'fase_extra', detalhe: null });
    expect(p?.chave).toMatch(/^[0-9a-f-]{36}$/);
  });

  it('a gerência vê só o agregado por protocolo e o NRD, sem paciente', async () => {
    estado.sessao = SESSOES.gerente;
    renderizar('/gestao-de-dose');
    const protocolos = await screen.findByRole('region', { name: 'Dose por protocolo' });
    expect(await within(protocolos).findByText('TC de tórax')).toBeInTheDocument();
    expect(within(protocolos).getByText(/CTDIvol mediano 10,6, 88% do NRD, 1 acima/)).toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: 'Exames' })).not.toBeInTheDocument();
    expect(screen.queryByText('Antônio Carlos Ribeiro')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cadastrar' })).toBeInTheDocument();
  });
});
