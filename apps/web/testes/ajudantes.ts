import { screen, within } from '@testing-library/react';
import type { UserEvent } from '@testing-library/user-event';

export const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/** Digita uma data `ddmmaaaa` no campo de data com o rótulo dado (DateField do React Aria). */
export async function digitarData(usuario: UserEvent, rotulo: string, digitos: string, dentro: HTMLElement = document.body): Promise<void> {
  const grupo = within(dentro).getByRole('group', { name: rotulo });
  const dia = within(grupo).getAllByRole('spinbutton')[0] as HTMLElement;
  await usuario.click(dia);
  await usuario.keyboard(digitos);
}

/** Endereço de volta que a tela mandou para a reautenticação (`voltar`), já decodificado. */
export function voltarDa(url: string): string {
  const u = new URL(url);
  expect(u.pathname).toBe('/v1/sessao/reautenticar');
  return u.searchParams.get('voltar') ?? '';
}

/** Mensagem R8 do servidor num alerta, com o código de suporte. */
export async function esperarAlertaDoServidor(detalhe: string, codigo: string, dentro?: HTMLElement): Promise<HTMLElement> {
  const alvo = dentro === undefined ? screen : within(dentro);
  const alertas = await alvo.findAllByRole('alert');
  const alerta = alertas.find((a) => a.textContent.includes(detalhe));
  expect(alerta, `alerta com "${detalhe}"`).toBeDefined();
  expect(alerta).toHaveTextContent(`Código de suporte: trace-${codigo}`);
  return alerta as HTMLElement;
}
