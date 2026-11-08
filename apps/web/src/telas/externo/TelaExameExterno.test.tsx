import { screen } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData, esperarAlertaDoServidor } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

async function preencher(usuario: UserEvent, nascimento: string) {
  await usuario.type(await screen.findByRole('searchbox', { name: 'Paciente do cadastro' }), 'Vera');
  await usuario.click(await screen.findByRole('button', { name: /^Vera Lúcia Antunes, nascimento 18\/09\/1957/ }));
  await usuario.type(screen.getByRole('textbox', { name: 'Instituição de origem' }), 'Hospital São Lucas');
  await usuario.type(screen.getByRole('textbox', { name: 'Nome no exame recebido' }), 'ANTUNES^VERA^L');
  await digitarData(usuario, 'Nascimento no exame recebido', nascimento);
  await usuario.type(screen.getByRole('textbox', { name: 'StudyUID' }), '1.2.3.4.5');
  await usuario.type(screen.getByRole('textbox', { name: 'Descrição' }), 'TC de crânio');
  await usuario.type(screen.getByRole('textbox', { name: 'Modalidade' }), 'ct');
  await digitarData(usuario, 'Data do estudo', '10052026');
  await usuario.type(screen.getByRole('textbox', { name: 'Imagens' }), '240');
  await usuario.type(screen.getByRole('textbox', { name: 'SHA-256 do conjunto' }), 'a'.repeat(64));
}

describe('Exame externo (Exame-Externo.dc.html)', () => {
  it('importa só depois de conferir o documento e mostra que é só leitura', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/exames-externos');
    await preencher(usuario, '18091957');
    await usuario.click(screen.getByRole('button', { name: 'Importar' }));
    expect(screen.getByText('Confira com o documento antes de importar.')).toBeInTheDocument();
    expect(pedidosDe('importar-externo')).toHaveLength(0);
    await usuario.click(screen.getByRole('checkbox', { name: 'Conferi o nome e o nascimento com o documento do paciente' }));
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('button', { name: 'Importar' }));
    expect(await screen.findByText(/EXT-2026-000001 importado como exame externo\./)).toBeInTheDocument();
    expect(pedidosDe('importar-externo')[0]?.corpo).toMatchObject({ nascimento_na_origem: '1957-09-18', documento_conferido: true, estudos: [{ modalidade: 'CT', imagens: 240 }] });
  });

  it('nascimento diferente é recusado pelo servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/exames-externos');
    await preencher(usuario, '19091957');
    await usuario.click(screen.getByRole('checkbox', { name: 'Conferi o nome e o nascimento com o documento do paciente' }));
    await usuario.click(screen.getByRole('button', { name: 'Importar' }));
    await esperarAlertaDoServidor('O nascimento no exame externo é diferente do cadastro.', 'nascimento-diferente');
  });
});
