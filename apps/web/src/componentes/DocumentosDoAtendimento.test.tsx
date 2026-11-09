import { screen, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../testes/axe';
import { caixaAberto, SESSOES, SONIA } from '../../testes/dados/fase1';
import { fase1 } from '../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../testes/nucleo-simulado';
import { renderizar } from '../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

async function abrirAntonio(usuario: UserEvent): Promise<HTMLElement> {
  await usuario.click(await screen.findByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954, 09:00/ }));
  return screen.getByRole('complementary', { name: 'Detalhe do agendamento' });
}

describe('Documentos do atendimento em PDF', () => {
  it('a recepção baixa o comprovante com o preparo pela gaveta do agendamento', async () => {
    const usuario = userEvent.setup();
    const { container, salvarArquivo } = renderizar('/agenda?data=2026-11-06');
    const gaveta = await abrirAntonio(usuario);
    const documentos = await within(gaveta).findByRole('region', { name: 'Documentos do paciente' });
    await usuario.click(within(documentos).getByRole('button', { name: 'Comprovante e preparo' }));
    const status = await within(documentos).findByText(/PDF baixado: AGD-2026-000123-CPV-2026-000004\.pdf\./);
    expect(status).toHaveTextContent('Emissão CPV-2026-000004 registrada');
    expect(salvarArquivo).toHaveBeenCalledTimes(1);
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('o técnico não vê os documentos (ato que o perfil nunca terá não aparece)', async () => {
    estado.sessao = SESSOES.tecnico;
    const usuario = userEvent.setup();
    renderizar('/agenda?data=2026-11-06');
    const gaveta = await abrirAntonio(usuario);
    expect(within(gaveta).getByText('Protocolo')).toBeInTheDocument();
    expect(within(gaveta).queryByRole('region', { name: 'Documentos do paciente' })).not.toBeInTheDocument();
  });

  it('o caixa baixa o recibo depois do pagamento confirmado', async () => {
    estado.sessao = SESSOES.caixa;
    fase1.caixa = caixaAberto();
    const usuario = userEvent.setup();
    const { salvarArquivo } = renderizar('/caixa?data=2026-11-06');
    const paciente = await screen.findByRole('combobox', { name: 'Paciente (check-in feito hoje)' });
    await screen.findByRole('option', { name: /^Sônia Maria Castro, 03\/02\/1962/ });
    await usuario.selectOptions(paciente, SONIA.atendimento_id ?? '');
    await usuario.selectOptions(screen.getByRole('combobox', { name: 'Forma de pagamento' }), 'dinheiro');
    await usuario.click(screen.getByRole('button', { name: 'Registrar recebimento' }));
    await usuario.click(await screen.findByRole('button', { name: 'Recibo' }));
    expect(await screen.findByText(/PDF baixado: REC-2026-000001-RCB-2026-000001\.pdf\./)).toBeInTheDocument();
    expect(salvarArquivo).toHaveBeenCalledTimes(1);
  });
});
