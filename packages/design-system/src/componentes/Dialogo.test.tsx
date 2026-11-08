import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { violacoesDeAcessibilidade } from '../../testes/axe';
import { Dialogo } from './Dialogo';

function Tela({ aoConfirmar, ocupado = false }: { aoConfirmar: () => void; ocupado?: boolean }) {
  const [aberto, setAberto] = useState(false);
  return (
    <>
      <button
        type="button"
        onClick={() => {
          setAberto(true);
        }}
      >
        Conferir e fazer check-in
      </button>
      <Dialogo
        aberto={aberto}
        titulo="Fazer o check-in de Antônio Carlos Ribeiro?"
        paciente={{ nome: 'Antônio Carlos Ribeiro', nascimento: '15/06/1954' }}
        rotuloConfirmar="Fazer check-in"
        aoConfirmar={aoConfirmar}
        aoFechar={() => {
          setAberto(false);
        }}
        ocupado={ocupado}
      >
        <p>Eco e Doppler de carótidas, Sala 1, 09:00.</p>
      </Dialogo>
    </>
  );
}

describe('Dialogo de confirmação', () => {
  it('mostra o quê e com quem, com nome e nascimento, e começa em Voltar', async () => {
    const usuario = userEvent.setup();
    render(<Tela aoConfirmar={vi.fn()} />);
    await usuario.click(screen.getByRole('button', { name: 'Conferir e fazer check-in' }));
    const dialogo = await screen.findByRole('alertdialog', { name: 'Fazer o check-in de Antônio Carlos Ribeiro?' });
    expect(dialogo).toHaveTextContent('Antônio Carlos Ribeiro, nascimento 15/06/1954');
    expect(dialogo).not.toHaveTextContent('Tem certeza');
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Voltar' })).toHaveFocus();
    });
  });

  it('prende o foco com Tab, fecha com Esc e devolve o foco a quem abriu', async () => {
    const usuario = userEvent.setup();
    render(<Tela aoConfirmar={vi.fn()} />);
    const abrir = screen.getByRole('button', { name: 'Conferir e fazer check-in' });
    await usuario.click(abrir);
    await screen.findByRole('alertdialog');
    await usuario.tab();
    expect(screen.getByRole('button', { name: 'Fazer check-in' })).toHaveFocus();
    await usuario.tab();
    expect(screen.getByRole('button', { name: 'Voltar' })).toHaveFocus();
    await usuario.keyboard('{Escape}');
    await waitFor(() => {
      expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    });
    await waitFor(() => {
      expect(abrir).toHaveFocus();
    });
  });

  it('confirma pelo botão que repete o verbo', async () => {
    const usuario = userEvent.setup();
    const aoConfirmar = vi.fn();
    render(<Tela aoConfirmar={aoConfirmar} />);
    await usuario.click(screen.getByRole('button', { name: 'Conferir e fazer check-in' }));
    await usuario.click(await screen.findByRole('button', { name: 'Fazer check-in' }));
    expect(aoConfirmar).toHaveBeenCalledTimes(1);
  });

  it('ocupado não fecha com Esc nem deixa voltar', async () => {
    const usuario = userEvent.setup();
    render(<Tela aoConfirmar={vi.fn()} ocupado />);
    await usuario.click(screen.getByRole('button', { name: 'Conferir e fazer check-in' }));
    await screen.findByRole('alertdialog');
    await usuario.keyboard('{Escape}');
    expect(screen.getByRole('alertdialog')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Voltar' })).toBeDisabled();
  });

  it('não tem violação de acessibilidade', async () => {
    const usuario = userEvent.setup();
    render(<Tela aoConfirmar={vi.fn()} />);
    await usuario.click(screen.getByRole('button', { name: 'Conferir e fazer check-in' }));
    await screen.findByRole('alertdialog');
    expect(await violacoesDeAcessibilidade(document.body)).toEqual([]);
  });
});
