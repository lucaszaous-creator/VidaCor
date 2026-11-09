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
  estado.sessao = SESSOES.gerente;
});

describe('Corpo clínico: CFM e certificado ICP-Brasil', () => {
  it('avisa o certificado perto de vencer e, sem CFM configurado, registra a consulta feita no portal', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/corpo-clinico');
    await usuario.click(await screen.findByRole('button', { name: /Dr\. Paulo Medeiros/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do médico' });
    expect(within(detalhe).getByText(/Certificado vence em 21 dias/)).toBeInTheDocument();
    await usuario.click(within(detalhe).getByRole('tab', { name: 'CFM e certificado' }));
    const consulta = within(detalhe).getByRole('region', { name: 'Consulta ao CFM' });
    await usuario.click(within(consulta).getByRole('button', { name: 'Consultar no CFM' }));
    const form = await within(consulta).findByRole('form', { name: 'Consulta feita no portal do CFM' });
    await usuario.click(within(form).getByRole('button', { name: 'Registrar consulta' }));
    expect(within(form).getByText('Escolha a situação que o portal mostrou.')).toBeInTheDocument();
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Situação no portal do CFM' }), 'regular');
    await usuario.type(within(form).getByRole('textbox', { name: 'Onde e como consultou' }), 'portal do CFM, busca por CRM e UF');
    await usuario.click(within(form).getByRole('button', { name: 'Registrar consulta' }));
    expect(await within(consulta).findByText(/portal do CFM, busca por CRM e UF\./)).toBeInTheDocument();
    expect(pedidosDe('consulta-crm').at(-1)?.corpo).toEqual({
      uf: 'SP',
      numero: '123456',
      fonte: 'portal_cfm',
      situacao: 'regular',
      observacao: 'portal do CFM, busca por CRM e UF',
    });
    const cert = within(detalhe).getByRole('region', { name: 'Certificado ICP-Brasil' });
    expect(within(cert).getByText('PAULO MEDEIROS, CPF ***.***.***-35')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });
});
