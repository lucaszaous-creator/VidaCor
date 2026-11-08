import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../testes/axe';
import { MenuLateral, type SecaoDoMenu } from './MenuLateral';

const SECOES: SecaoDoMenu[] = [
  {
    id: 'clinica',
    titulo: 'Clínica',
    itens: [
      { id: 'agenda', rotulo: 'Agenda', href: '/agenda', icone: 'agenda', cor: 'blue', contador: 42 },
      { id: 'recepcao', rotulo: 'Recepção e senhas', href: '/recepcao', icone: 'recepcao', cor: 'teal' },
    ],
  },
  {
    id: 'exames',
    titulo: 'Exames',
    itens: [
      {
        id: 'laudos',
        rotulo: 'Laudar',
        href: '/laudos',
        icone: 'imagem',
        cor: 'violet',
        alerta: { numero: 11, rotulo: '11 laudos pendentes' },
      },
    ],
  },
];

function Casca() {
  const [recolhido, setRecolhido] = useState(false);
  return (
    <MenuLateral
      secoes={SECOES}
      ativo="agenda"
      recolhido={recolhido}
      aoAlternar={() => {
        setRecolhido((r) => !r);
      }}
      marca={{ nome: 'VidaCor', unidade: 'Unidade Centro', href: '/' }}
      usuario={{ nome: 'Ana Souza', papel: 'Recepção' }}
    />
  );
}

describe('MenuLateral', () => {
  it('marca a tela atual e abre só o grupo dela', () => {
    render(<Casca />);
    expect(screen.getByRole('link', { name: /Agenda/ })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('button', { name: /Clínica/ })).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('button', { name: /Exames/ })).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('link', { name: /Laudar/ })).not.toBeInTheDocument();
  });

  it('abre e fecha um grupo pelo teclado', async () => {
    const usuario = userEvent.setup();
    render(<Casca />);
    const exames = screen.getByRole('button', { name: /Exames/ });
    exames.focus();
    await usuario.keyboard('{Enter}');
    expect(exames).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('link', { name: /Laudar/ })).toBeInTheDocument();
    expect(screen.getByLabelText('11 laudos pendentes')).toHaveTextContent('11');
  });

  it('recolhe para só ícones com aria-expanded e mantém o nome de cada item', async () => {
    const usuario = userEvent.setup();
    const { container } = render(<Casca />);
    const recolher = screen.getByRole('button', { name: 'Recolher menu' });
    expect(recolher).toHaveAttribute('aria-expanded', 'true');
    await usuario.click(recolher);
    expect(container.querySelector('.sb-mini')).not.toBeNull();
    const expandir = screen.getByRole('button', { name: 'Expandir menu' });
    expect(expandir).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getByRole('link', { name: 'Agenda, 42' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('link', { name: 'Laudar, 11 laudos pendentes' })).toBeInTheDocument();
    await usuario.click(expandir);
    expect(container.querySelector('.sb-mini')).toBeNull();
  });

  it('não tem violação de acessibilidade, aberto e recolhido', async () => {
    const usuario = userEvent.setup();
    const { container } = render(<Casca />);
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('button', { name: 'Recolher menu' }));
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });
});
