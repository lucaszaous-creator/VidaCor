import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { violacoesDeAcessibilidade } from '../../testes/axe';
import { formatarDataHora, formatarDinheiro, formatarPontosBase, lerPercentual, lerReais } from '../formatos';
import { DialogoDeFormulario } from './DialogoDeFormulario';
import { EtiquetaPulseira } from './EtiquetaPulseira';
import { AreaDeTexto, CaixaDeMarcar, Escolha, ListaDeDados, SimOuNao } from './Formulario';

describe('formatos de leitura da Fase 1', () => {
  it('dinheiro em centavos vira reais, sem conta de negócio', () => {
    expect(formatarDinheiro(38000)).toBe('R$ 380,00');
    expect(formatarDinheiro(36100)).toBe('R$ 361,00');
    expect(formatarDinheiro(-1500)).toBe('-R$ 15,00');
    expect(formatarDinheiro(1.5)).toBe('1.5');
  });

  it('pontos base viram percentual', () => {
    expect(formatarPontosBase(500)).toBe('5,00%');
    expect(formatarPontosBase(1500)).toBe('15,00%');
  });

  it('data e hora local de um instante', () => {
    expect(formatarDataHora('2026-11-06T13:42:00Z', 'America/Sao_Paulo')).toBe('06/11/2026 10:42');
    expect(formatarDataHora('ontem')).toBe('ontem');
  });

  it('lê reais digitados no formato brasileiro', () => {
    expect(lerReais('380')).toBe(38000);
    expect(lerReais('380,5')).toBe(38050);
    expect(lerReais('R$ 1.380,50')).toBe(138050);
    expect(lerReais('3,805')).toBeNull();
    expect(lerReais('abc')).toBeNull();
    expect(lerReais('')).toBeNull();
  });

  it('lê percentual digitado em pontos base', () => {
    expect(lerPercentual('15')).toBe(1500);
    expect(lerPercentual('5,5 %')).toBe(550);
    expect(lerPercentual('-5')).toBeNull();
  });
});

function SimOuNaoControlado({ aoMudar }: { aoMudar: (v: boolean) => void }) {
  const [valor, setValor] = useState<boolean | null>(null);
  return (
    <SimOuNao
      pergunta="Já teve reação a contraste iodado?"
      valor={valor}
      aoMudar={(v) => {
        setValor(v);
        aoMudar(v);
      }}
    />
  );
}

describe('controles de formulário', () => {
  it('sim ou não começa sem resposta e responde ao teclado', async () => {
    const usuario = userEvent.setup();
    const aoMudar = vi.fn();
    const { container } = render(<SimOuNaoControlado aoMudar={aoMudar} />);
    const grupo = screen.getByRole('radiogroup', { name: 'Já teve reação a contraste iodado?' });
    expect(grupo).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'Sim' })).not.toBeChecked();
    expect(screen.getByRole('radio', { name: 'Não' })).not.toBeChecked();
    await usuario.click(screen.getByRole('radio', { name: 'Não' }));
    expect(aoMudar).toHaveBeenLastCalledWith(false);
    await usuario.keyboard('{ArrowLeft}');
    expect(aoMudar).toHaveBeenLastCalledWith(true);
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('caixa de marcar com rótulo, erro em texto e espaço para marcar', async () => {
    const usuario = userEvent.setup();
    const aoMudar = vi.fn();
    const { container } = render(
      <CaixaDeMarcar rotulo="Documento com foto conferido" marcado={false} aoMudar={aoMudar} erro="Confira o documento com foto." />,
    );
    const caixa = screen.getByRole('checkbox', { name: 'Documento com foto conferido' });
    caixa.focus();
    await usuario.keyboard(' ');
    expect(aoMudar).toHaveBeenCalledWith(true);
    expect(screen.getByText('Confira o documento com foto.')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('área de texto e escolha com rótulo visível', async () => {
    const aoMudar = vi.fn();
    const { container } = render(
      <>
        <AreaDeTexto rotulo="Motivo" erro="Escreva o motivo." />
        <Escolha
          rotulo="Nível"
          valor=""
          aoMudar={aoMudar}
          vazio="Escolha o nível"
          opcoes={[{ valor: 'total', rotulo: 'Total' }]}
          erro="Escolha o nível."
        />
        <ListaDeDados itens={[{ rotulo: 'Accession', valor: 'ACC-2026-052311' }]} />
      </>,
    );
    expect(screen.getByRole('textbox', { name: 'Motivo' })).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Nível' })).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByText('ACC-2026-052311')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });
});

describe('Etiqueta da pulseira', () => {
  it('leva só nome, nascimento e códigos', async () => {
    const { container } = render(
      <EtiquetaPulseira nome="Antônio Carlos Ribeiro" nascimento="15/06/1954" codigo="ETQ-2026-018742" accession="ACC-2026-052311" situacao="valida" />,
    );
    const etiqueta = screen.getByRole('figure', { name: 'Pulseira ETQ-2026-018742, válida' });
    expect(etiqueta).toHaveTextContent('ANTÔNIO CARLOS RIBEIRO');
    expect(etiqueta).toHaveTextContent('Nasc. 15/06/1954');
    expect(etiqueta).toHaveTextContent('ACC-2026-052311');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('invalidada mostra o motivo e quem a substituiu', () => {
    render(
      <EtiquetaPulseira
        nome="Antônio Carlos Ribeiro"
        nascimento="15/06/1954"
        codigo="ETQ-2026-018742"
        situacao="invalidada"
        motivo="rasgou"
        substituidaPor="ETQ-2026-018790"
      />,
    );
    expect(screen.getByRole('figure', { name: 'Pulseira ETQ-2026-018742, invalidada' })).toHaveTextContent(
      'Invalidada: rasgou. Substituída por ETQ-2026-018790. Se for bipada, o sistema bloqueia.',
    );
  });
});

describe('Diálogo de formulário', () => {
  it('envia com Enter, mostra os dois identificadores e volta com Esc', async () => {
    const usuario = userEvent.setup();
    const aoConfirmar = vi.fn();
    const aoFechar = vi.fn();
    render(
      <DialogoDeFormulario
        aberto
        irreversivel
        titulo="Reimprimir a pulseira de Antônio Carlos Ribeiro?"
        paciente={{ nome: 'Antônio Carlos Ribeiro', nascimento: '15/06/1954' }}
        rotuloConfirmar="Reimprimir"
        aoConfirmar={aoConfirmar}
        aoFechar={aoFechar}
      >
        <label>
          Motivo
          <input name="motivo" />
        </label>
      </DialogoDeFormulario>,
    );
    const dialogo = screen.getByRole('alertdialog', { name: 'Reimprimir a pulseira de Antônio Carlos Ribeiro?' });
    expect(dialogo).toHaveTextContent('Antônio Carlos Ribeiro, nascimento 15/06/1954');
    await usuario.type(screen.getByRole('textbox', { name: 'Motivo' }), 'rasgou{Enter}');
    expect(aoConfirmar).toHaveBeenCalledTimes(1);
    await usuario.keyboard('{Escape}');
    expect(aoFechar).toHaveBeenCalled();
    expect(await violacoesDeAcessibilidade(dialogo)).toEqual([]);
  });
});
