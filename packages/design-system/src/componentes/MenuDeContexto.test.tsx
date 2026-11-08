import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { violacoesDeAcessibilidade } from '../../testes/axe';
import { BotaoDeIcone } from './Botao';
import { MenuDeContexto, useMenuDeContexto, type AcaoDoMenu } from './MenuDeContexto';

// Ações de exemplo só para testar o componente: na aplicação a lista vem do servidor.
const ACOES: AcaoDoMenu[] = [
  { id: 'abrir', rotulo: 'Abrir', estado: 'disponivel', atalho: 'Enter' },
  { id: 'confirmar', rotulo: 'Confirmar presença', estado: 'indisponivel', motivo: 'Já confirmado pelo WhatsApp' },
  { id: 'remarcar', rotulo: 'Remarcar', estado: 'disponivel' },
  { id: 'cancelar', rotulo: 'Cancelar agendamento', estado: 'disponivel', perigo: true },
];

function Objeto({ acoes, aoEscolher }: { acoes: AcaoDoMenu[]; aoEscolher: (id: string) => void }) {
  const menu = useMenuDeContexto();
  return (
    <div>
      <button type="button" {...menu.propsDoAlvo}>
        Antônio Carlos Ribeiro, 09:00
      </button>
      <BotaoDeIcone
        icone="maisOpcoes"
        estilo="mais"
        rotulo="Mais opções de Antônio Carlos Ribeiro"
        onPress={(e) => {
          menu.abrirEm(e.target as HTMLElement);
        }}
      />
      <MenuDeContexto
        estado={menu}
        cabecalho={{ titulo: 'Antônio Carlos Ribeiro', subtitulo: '15/06/1954' }}
        acoes={acoes}
        motivoSemAcoes="Disponível na próxima entrega"
        aoEscolher={aoEscolher}
      />
    </div>
  );
}

describe('MenuDeContexto', () => {
  it('abre com Shift+F10, mostra nome e nascimento e percorre com as setas pulando o indisponível', async () => {
    const usuario = userEvent.setup();
    const aoEscolher = vi.fn();
    render(<Objeto acoes={ACOES} aoEscolher={aoEscolher} />);
    const alvo = screen.getByRole('button', { name: 'Antônio Carlos Ribeiro, 09:00' });
    alvo.focus();
    await usuario.keyboard('{Shift>}{F10}{/Shift}');
    const menu = await screen.findByRole('menu', { name: 'Ações de Antônio Carlos Ribeiro' });
    expect(screen.getByText('15/06/1954')).toBeInTheDocument();
    const itens = within(menu).getAllByRole('menuitem');
    expect(itens.map((i) => i.textContent)).toEqual([
      'AbrirEnter',
      'Confirmar presençaJá confirmado pelo WhatsApp',
      'Remarcar',
      'Cancelar agendamento',
    ]);
    await waitFor(() => {
      expect(itens[0]).toHaveFocus();
    });
    expect(itens[1]).toHaveAttribute('aria-disabled', 'true');
    await usuario.keyboard('{ArrowDown}');
    expect(itens[2]).toHaveFocus();
    await usuario.keyboard('{Enter}');
    expect(aoEscolher).toHaveBeenCalledWith('remarcar');
    await waitFor(() => {
      expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    });
  });

  it('abre pela tecla Menu e fecha com Esc devolvendo o foco ao objeto', async () => {
    const usuario = userEvent.setup();
    render(<Objeto acoes={ACOES} aoEscolher={vi.fn()} />);
    const alvo = screen.getByRole('button', { name: 'Antônio Carlos Ribeiro, 09:00' });
    alvo.focus();
    await usuario.keyboard('{ContextMenu}');
    await screen.findByRole('menu');
    await usuario.keyboard('{Escape}');
    await waitFor(() => {
      expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    });
    expect(alvo).toHaveFocus();
  });

  it('abre pelo clique direito sem o menu do navegador', async () => {
    const usuario = userEvent.setup();
    render(<Objeto acoes={ACOES} aoEscolher={vi.fn()} />);
    await usuario.pointer({ keys: '[MouseRight]', target: screen.getByRole('button', { name: 'Antônio Carlos Ribeiro, 09:00' }) });
    expect(await screen.findByRole('menu')).toBeInTheDocument();
  });

  it('abre pelo botão de mais opções', async () => {
    const usuario = userEvent.setup();
    render(<Objeto acoes={ACOES} aoEscolher={vi.fn()} />);
    await usuario.click(screen.getByRole('button', { name: 'Mais opções de Antônio Carlos Ribeiro' }));
    expect(await screen.findByRole('menu')).toBeInTheDocument();
  });

  it('não executa ação indisponível', async () => {
    const usuario = userEvent.setup();
    const aoEscolher = vi.fn();
    render(<Objeto acoes={ACOES} aoEscolher={aoEscolher} />);
    await usuario.click(screen.getByRole('button', { name: 'Mais opções de Antônio Carlos Ribeiro' }));
    await usuario.click(await screen.findByRole('menuitem', { name: /Confirmar presença/ }));
    expect(aoEscolher).not.toHaveBeenCalled();
  });

  it('põe a destrutiva no fim, separada por linha', async () => {
    const usuario = userEvent.setup();
    const fora = [ACOES[3], ACOES[0]].filter((a): a is AcaoDoMenu => a !== undefined);
    render(<Objeto acoes={fora} aoEscolher={vi.fn()} />);
    await usuario.click(screen.getByRole('button', { name: 'Mais opções de Antônio Carlos Ribeiro' }));
    const menu = await screen.findByRole('menu');
    expect(within(menu).getAllByRole('menuitem').map((i) => i.textContent)).toEqual(['AbrirEnter', 'Cancelar agendamento']);
    expect(within(menu).getByRole('separator')).toBeInTheDocument();
  });

  it('sem ações do servidor, mostra o motivo e não inventa nenhuma', async () => {
    const usuario = userEvent.setup();
    const aoEscolher = vi.fn();
    render(<Objeto acoes={[]} aoEscolher={aoEscolher} />);
    const alvo = screen.getByRole('button', { name: 'Antônio Carlos Ribeiro, 09:00' });
    alvo.focus();
    await usuario.keyboard('{Shift>}{F10}{/Shift}');
    const menu = await screen.findByRole('menu');
    const itens = within(menu).getAllByRole('menuitem');
    expect(itens).toHaveLength(1);
    expect(itens[0]).toHaveAttribute('aria-disabled', 'true');
    expect(itens[0]).toHaveTextContent('Nenhuma ação disponível');
    expect(itens[0]).toHaveTextContent('Disponível na próxima entrega');
    await usuario.keyboard('{Enter}');
    expect(aoEscolher).not.toHaveBeenCalled();
    await usuario.keyboard('{Escape}');
    await waitFor(() => {
      expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    });
    expect(alvo).toHaveFocus();
  });

  it('não tem violação de acessibilidade com o menu aberto', async () => {
    const usuario = userEvent.setup();
    render(<Objeto acoes={ACOES} aoEscolher={vi.fn()} />);
    await usuario.click(screen.getByRole('button', { name: 'Mais opções de Antônio Carlos Ribeiro' }));
    await screen.findByRole('menu');
    expect(await violacoesDeAcessibilidade(document.body)).toEqual([]);
  });
});
