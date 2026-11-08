import { CalendarDate } from '@internationalized/date';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { violacoesDeAcessibilidade } from '../../testes/axe';
import { Abas } from './Abas';
import { Avatar, iniciais } from './Avatar';
import { Aviso } from './Aviso';
import { BarraDoPaciente } from './BarraDoPaciente';
import { Botao, BotaoDeIcone } from './Botao';
import { Busca, Campo, CampoDeData } from './Campo';
import { Card } from './Card';
import { Chip } from './Chip';
import { Carregando, ErroTela, SemConexao, SemPermissao, Vazio } from './Estados';
import { Gaveta } from './Gaveta';
import { Tabela } from './Tabela';

describe('Botao', () => {
  it('usa as classes do vc10 e responde ao clique', async () => {
    const usuario = userEvent.setup();
    const aoApertar = vi.fn();
    render(
      <Botao variante="principal" onPress={aoApertar}>
        Fazer check-in
      </Botao>,
    );
    const botao = screen.getByRole('button', { name: 'Fazer check-in' });
    expect(botao).toHaveClass('btn', 'btn-p');
    await usuario.click(botao);
    expect(aoApertar).toHaveBeenCalledTimes(1);
  });

  it('ocupado não aceita segundo clique', async () => {
    const usuario = userEvent.setup();
    const aoApertar = vi.fn();
    render(
      <Botao variante="principal" ocupado onPress={aoApertar}>
        Fazer check-in
      </Botao>,
    );
    await usuario.click(screen.getByRole('button', { name: /Fazer check-in/ }));
    expect(aoApertar).not.toHaveBeenCalled();
  });

  it('botão só com ícone exige nome', () => {
    render(<BotaoDeIcone icone="fechar" estilo="fechar" rotulo="Fechar detalhe" />);
    expect(screen.getByRole('button', { name: 'Fechar detalhe' })).toHaveClass('xb');
  });
});

describe('Chip, Avatar e Card', () => {
  it('chip é cor mais texto', () => {
    render(<Chip cor="violeta" texto="Em atendimento" />);
    expect(screen.getByText('Em atendimento')).toHaveClass('chip', 'c-violet');
  });

  it('avatar mostra as iniciais e fica fora do leitor de tela', () => {
    expect(iniciais('Antônio Carlos Ribeiro')).toBe('AR');
    expect(iniciais('Ana')).toBe('A');
    const { container } = render(<Avatar nome="Sônia Maria Castro" />);
    expect(container.firstElementChild).toHaveAttribute('aria-hidden', 'true');
    expect(container.firstElementChild).toHaveTextContent('SC');
  });

  it('card com título nomeia a região', () => {
    render(
      <Card titulo="Agenda do dia">
        <p>conteúdo</p>
      </Card>,
    );
    expect(screen.getByRole('region', { name: 'Agenda do dia' })).toHaveClass('card');
  });
});

describe('Aviso em três níveis', () => {
  it('bloqueante é alerta, com código de suporte', () => {
    render(
      <Aviso nivel="bloqueante" codigoDeSuporte="4bf92f35">
        O nascimento informado não confere com o cadastro. Nada foi gravado.
      </Aviso>,
    );
    const alerta = screen.getByRole('alert');
    expect(alerta).toHaveClass('banner', 'err');
    expect(alerta).toHaveTextContent('Bloqueado:');
    expect(alerta).toHaveTextContent('Código de suporte: 4bf92f35');
  });

  it('atenção e informativo são status, com classes distintas', () => {
    render(
      <>
        <Aviso nivel="atencao">Precisa de ação hoje.</Aviso>
        <Aviso nivel="informativo">Contexto útil.</Aviso>
      </>,
    );
    const [atencao, info] = screen.getAllByRole('status');
    expect(atencao).toHaveClass('banner', 'off');
    expect(atencao).toHaveTextContent('Atenção:');
    expect(info).toHaveClass('banner', 'info');
  });
});

describe('Estados de tela', () => {
  it('cada estado diz o que é', async () => {
    const { container } = render(
      <div>
        <Vazio titulo="Nenhum agendamento neste dia" />
        <Carregando rotulo="Carregando a agenda" />
        <SemConexao desde="10:12">O que foi digitado continua aqui.</SemConexao>
        <ErroTela mensagem="A agenda não carregou. Nada foi alterado. Tente de novo." codigoDeSuporte="abc123" />
        <SemPermissao titulo="Você não tem acesso à agenda" detalhe="Peça à coordenação." />
      </div>,
    );
    expect(screen.getByText('Nenhum agendamento neste dia')).toBeInTheDocument();
    expect(screen.getByRole('status', { name: 'Carregando a agenda' })).toBeInTheDocument();
    expect(screen.getByText(/Sem conexão com o servidor desde 10:12/)).toBeInTheDocument();
    expect(screen.getByRole('alert')).toHaveTextContent('Código de suporte: abc123');
    expect(screen.getByText('Você não tem acesso à agenda')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });
});

describe('Campos', () => {
  it('campo de texto com rótulo, ajuda e erro ligado ao campo, mantendo o valor', async () => {
    const usuario = userEvent.setup();
    function C() {
      const [v, setV] = useState('');
      return <Campo rotulo="Nome completo" ajuda="Como no documento." erro={v === 'x' ? 'Nome incompleto.' : null} value={v} onChange={setV} />;
    }
    const { container } = render(<C />);
    const campo = screen.getByRole('textbox', { name: 'Nome completo' });
    await usuario.type(campo, 'x');
    expect(campo).toHaveValue('x');
    expect(campo).toHaveAttribute('aria-invalid', 'true');
    expect(campo).toHaveAccessibleDescription(/Nome incompleto\./);
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('data em pt-BR na ordem dia, mês e ano, digitável', async () => {
    const usuario = userEvent.setup();
    const aoMudar = vi.fn();
    function C() {
      const [v, setV] = useState<CalendarDate | null>(null);
      return (
        <CampoDeData
          rotulo="Data de nascimento"
          valor={v}
          aoMudar={(d) => {
            setV(d);
            aoMudar(d?.toString() ?? null);
          }}
        />
      );
    }
    const { container } = render(<C />);
    const grupo = screen.getByRole('group', { name: 'Data de nascimento' });
    const segmentos = within(grupo).getAllByRole('spinbutton');
    expect(segmentos.map((s) => s.getAttribute('data-type'))).toEqual(['day', 'month', 'year']);
    await usuario.click(segmentos[0] as HTMLElement);
    await usuario.keyboard('15061954');
    expect(aoMudar).toHaveBeenLastCalledWith('1954-06-15');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('busca com rótulo para leitor de tela', async () => {
    const usuario = userEvent.setup();
    const aoMudar = vi.fn();
    render(<Busca rotulo="Buscar paciente" valor="" aoMudar={aoMudar} atalho="Ctrl K" />);
    await usuario.type(screen.getByRole('searchbox', { name: 'Buscar paciente' }), 'M');
    expect(aoMudar).toHaveBeenCalledWith('M');
  });
});

describe('Tabela e Abas', () => {
  it('tabela com cabeçalho real e abertura por teclado', async () => {
    const usuario = userEvent.setup();
    const aoAbrir = vi.fn();
    const { container } = render(
      <Tabela
        rotulo="Pacientes do dia"
        colunas={[
          { id: 'nome', rotulo: 'Paciente', cabecalhoDaLinha: true, celula: (l: { id: string; nome: string; hora: string }) => l.nome },
          { id: 'hora', rotulo: 'Hora', semQuebra: true, celula: (l) => l.hora },
        ]}
        linhas={[
          { id: '1', nome: 'Maria Helena Duarte', hora: '07:00' },
          { id: '2', nome: 'Sônia Maria Castro', hora: '08:00' },
        ]}
        aoAbrir={aoAbrir}
      />,
    );
    expect(screen.getAllByRole('columnheader').map((c) => c.textContent)).toEqual(['Paciente', 'Hora']);
    const linhas = screen.getAllByRole('row');
    await usuario.click(within(linhas[2] as HTMLElement).getByText('Sônia Maria Castro'));
    expect(aoAbrir).toHaveBeenCalledWith({ id: '2', nome: 'Sônia Maria Castro', hora: '08:00' });
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('tabela vazia diz que está vazia', () => {
    render(<Tabela rotulo="Pacientes do dia" colunas={[{ id: 'n', rotulo: 'Paciente', cabecalhoDaLinha: true, celula: (l: { id: string }) => l.id }]} linhas={[]} vazio="Ninguém hoje." />);
    expect(screen.getByText('Ninguém hoje.')).toBeInTheDocument();
  });

  it('abas trocam com as setas', async () => {
    const usuario = userEvent.setup();
    const { container } = render(
      <Abas
        rotulo="Visualização"
        abas={[
          { id: 'dia', rotulo: 'Dia', conteudo: <p>Grade do dia</p> },
          { id: 'semana', rotulo: 'Semana', conteudo: <p>Resumo da semana</p> },
        ]}
      />,
    );
    const [dia, semana] = screen.getAllByRole('tab');
    expect(dia).toHaveAttribute('aria-selected', 'true');
    await usuario.click(dia as HTMLElement);
    await usuario.keyboard('{ArrowRight}');
    expect(semana).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tabpanel')).toHaveTextContent('Resumo da semana');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });
});

describe('BarraDoPaciente e Gaveta', () => {
  it('barra mostra os dois identificadores', async () => {
    const { container } = render(<BarraDoPaciente nome="Sônia Maria Castro" nascimento="03/02/1962" resumo="Ecocardiograma, Sala 1" />);
    const barra = screen.getByRole('region', { name: 'Paciente em foco: Sônia Maria Castro, nascimento 03/02/1962' });
    expect(barra).toHaveTextContent('Sônia Maria Castro');
    expect(barra).toHaveTextContent('03/02/1962');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('gaveta fecha com Esc e pelo botão', async () => {
    const usuario = userEvent.setup();
    const aoFechar = vi.fn();
    render(
      <Gaveta titulo="Antônio Carlos Ribeiro" subTitulo="15/06/1954" rotulo="Detalhe do agendamento" aoFechar={aoFechar}>
        <button type="button">dentro</button>
      </Gaveta>,
    );
    expect(screen.getByRole('complementary', { name: 'Detalhe do agendamento' })).toHaveTextContent('15/06/1954');
    screen.getByRole('button', { name: 'dentro' }).focus();
    await usuario.keyboard('{Escape}');
    await usuario.click(screen.getByRole('button', { name: 'Fechar detalhe' }));
    expect(aoFechar).toHaveBeenCalledTimes(2);
  });
});
