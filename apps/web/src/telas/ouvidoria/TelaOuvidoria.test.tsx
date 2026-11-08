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
});

describe('Ouvidoria (Ouvidoria.dc.html)', () => {
  it('recepção só registra e recebe o protocolo com o prazo do servidor', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/ouvidoria');
    const form = await screen.findByRole('form', { name: 'Registrar manifestação' });
    expect(screen.queryByRole('region', { name: 'Manifestações' })).not.toBeInTheDocument();
    await usuario.click(within(form).getByRole('button', { name: 'Registrar e gerar protocolo' }));
    expect(within(form).getByText('Escolha o paciente ou informe o nome.')).toBeInTheDocument();
    expect(pedidosDe('registrar-manifestacao')).toHaveLength(0);
    await usuario.type(within(form).getByRole('textbox', { name: 'Ou nome de quem se manifestou' }), 'Joana Acompanhante');
    await usuario.type(within(form).getByRole('textbox', { name: 'O que a pessoa relatou' }), 'Esperou muito na recepção sem aviso.');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(form).getByRole('button', { name: 'Registrar e gerar protocolo' }));
    expect(await screen.findByText('Manifestação registrada: protocolo OUV-2026-000002.')).toBeInTheDocument();
    expect(pedidosDe('registrar-manifestacao')[0]?.corpo).toMatchObject({ tipo: 'reclamacao', nome_informado: 'Joana Acompanhante', anonima: false });
  });

  it('gerência vê a manifestação com dois identificadores e responde', async () => {
    estado.sessao = SESSOES.gerente;
    const usuario = userEvent.setup();
    renderizar('/ouvidoria');
    await usuario.click(await screen.findByRole('button', { name: /Reclamação, Maria Helena Duarte/ }));
    const d = screen.getByRole('region', { name: 'Detalhe da manifestação' });
    expect(within(d).getByText('Maria Helena Duarte, nascimento 12/03/1968')).toBeInTheDocument();
    await usuario.type(within(d).getByRole('textbox', { name: 'Resposta' }), 'Reforçamos a equipe e avisaremos a espera prevista.');
    await usuario.click(within(d).getByRole('button', { name: 'Enviar resposta' }));
    expect(await within(d).findByText('Resposta enviada.')).toBeInTheDocument();
  });
});
