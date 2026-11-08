import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { ANTONIO } from '../../../testes/dados/ficticios';
import { estado, problema, reiniciarNucleo, servidor } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

const URL_DO_DIA = '/agenda?data=2026-11-06';
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

beforeEach(() => {
  reiniciarNucleo();
});

async function abrirAntonio(usuario: UserEvent): Promise<HTMLElement> {
  await usuario.click(await screen.findByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954, 09:00/ }));
  return screen.getByRole('complementary', { name: 'Detalhe do agendamento' });
}

async function preencherEConferir(usuario: UserEvent, nome: string, nascimento: string): Promise<void> {
  const gaveta = screen.getByRole('complementary', { name: 'Check-in do paciente' });
  const campoNome = within(gaveta).getByRole('textbox', { name: 'Nome completo' });
  await usuario.clear(campoNome);
  await usuario.type(campoNome, nome);
  const dia = within(gaveta).getAllByRole('spinbutton')[0] as HTMLElement;
  await usuario.click(dia);
  await usuario.keyboard(nascimento);
  await usuario.click(within(gaveta).getByRole('button', { name: 'Conferir e fazer check-in' }));
}

describe('Agenda do dia', () => {
  it('mostra o dia, as salas e cada paciente com nome e nascimento', async () => {
    const { container } = renderizar(URL_DO_DIA);
    expect(await screen.findByRole('heading', { name: 'Sexta, 6 de novembro' })).toBeInTheDocument();
    expect(await screen.findByText('Sala 1, eco', { selector: '.gh div' })).toBeInTheDocument();
    expect(screen.getByText('38')).toBeInTheDocument();
    const antonio = screen.getByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954, 09:00, Eco e Doppler de carótidas, Confirmado$/ });
    expect(antonio).toHaveTextContent('Antônio C. Ribeiro');
    expect(screen.getByRole('link', { name: /Agenda e check-in/ })).toHaveAttribute('aria-current', 'page');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('dia sem agendamento mostra o vazio, nunca uma grade vazia sem explicação', async () => {
    renderizar('/agenda?data=2026-11-08');
    expect(await screen.findByText('Nenhum agendamento neste dia')).toBeInTheDocument();
  });

  it('erro do servidor aparece com a mensagem dele e o código de suporte', async () => {
    servidor.use(http.get('*/v1/agenda', () => problema(500, 'erro-interno', 'A agenda não pôde ser lida agora. Nada foi alterado. Tente de novo em instantes.')));
    renderizar(URL_DO_DIA);
    const alerta = await screen.findByRole('alert');
    expect(alerta).toHaveTextContent('A agenda não pôde ser lida agora.');
    expect(alerta).toHaveTextContent('Código de suporte: trace-erro-interno');
    expect(screen.queryByText('38')).not.toBeInTheDocument();
  });

  it('perfil sem acesso vê o estado de sem permissão', async () => {
    servidor.use(http.get('*/v1/agenda', () => problema(403, 'sem-permissao', 'Seu perfil não pode ver a agenda.')));
    renderizar(URL_DO_DIA);
    expect(await screen.findByText('Você não tem acesso à agenda')).toBeInTheDocument();
  });

  it('menu de contexto do agendamento abre com Shift+F10 e não inventa ações', async () => {
    const usuario = userEvent.setup();
    renderizar(URL_DO_DIA);
    const alvo = await screen.findByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento/ });
    alvo.focus();
    await usuario.keyboard('{Shift>}{F10}{/Shift}');
    const menu = await screen.findByRole('menu', { name: 'Ações de Antônio Carlos Ribeiro' });
    expect(screen.getByText(/Nascimento 15\/06\/1954, 09:00, Eco e Doppler de carótidas/)).toBeInTheDocument();
    const itens = within(menu).getAllByRole('menuitem');
    expect(itens).toHaveLength(1);
    expect(itens[0]).toHaveAttribute('aria-disabled', 'true');
    expect(itens[0]).toHaveTextContent('Disponível na próxima entrega');
    await usuario.keyboard('{Escape}');
    await waitFor(() => {
      expect(alvo).toHaveFocus();
    });
  });

  it('avisa quando o estudo chega pelo SSE, com nome e nascimento', async () => {
    const { fontes } = renderizar(URL_DO_DIA);
    await screen.findByRole('heading', { name: 'Sexta, 6 de novembro' });
    await screen.findByText('38');
    const fonte = fontes[0];
    expect(fonte?.url).toBe(`${window.location.origin}/v1/eventos`);
    const sonia = estado.itens.find((i) => i.paciente.nome === 'Sônia Maria Castro');
    act(() => {
      fonte?.abrir();
      fonte?.emitir('estudo-recebido', { atendimento_id: sonia?.atendimento_id, accession: 'ACC-2026-052301', imagens: 24 });
    });
    const aviso = await screen.findByText(/Estudo recebido\./);
    expect(aviso.closest('[role="status"]')).toHaveTextContent(
      'Sônia Maria Castro, nascimento 03/02/1962. Accession ACC-2026-052301, 24 imagens.',
    );
  });

  it('avisa quando o tempo real cai, sem parar a agenda', async () => {
    const { fontes } = renderizar(URL_DO_DIA);
    await screen.findByText('38');
    act(() => {
      fontes[0]?.cair();
    });
    expect(await screen.findByText(/Avisos em tempo real fora do ar\./)).toBeInTheDocument();
    act(() => {
      fontes[0]?.abrir();
    });
    await waitFor(() => {
      expect(screen.queryByText(/Avisos em tempo real fora do ar\./)).not.toBeInTheDocument();
    });
  });
});

describe('Check-in com dois identificadores', () => {
  it('caminho feliz: confere, confirma com nome e nascimento e mostra atendimento e accession', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar(URL_DO_DIA);
    const detalhe = await abrirAntonio(usuario);
    expect(detalhe).toHaveTextContent('Antônio Carlos Ribeiro');
    expect(detalhe).toHaveTextContent('Nascimento 15/06/1954');
    expect(detalhe).toHaveTextContent('09:00 às 09:30, Sala 1, eco');
    await usuario.click(within(detalhe).getByRole('button', { name: 'Fazer check-in' }));

    const gaveta = screen.getByRole('complementary', { name: 'Check-in do paciente' });
    expect(within(gaveta).getByRole('textbox', { name: 'Nome completo' })).toHaveValue('');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await preencherEConferir(usuario, 'Antônio Carlos Ribeiro', '15061954');

    const dialogo = await screen.findByRole('alertdialog', { name: 'Fazer o check-in de Antônio Carlos Ribeiro?' });
    expect(dialogo).toHaveTextContent('Antônio Carlos Ribeiro, nascimento 15/06/1954');
    expect(estado.pedidosDeCheckin).toHaveLength(0);
    await usuario.click(within(dialogo).getByRole('button', { name: 'Fazer check-in' }));

    const feito = await screen.findByRole('complementary', { name: 'Check-in feito' });
    expect(within(feito).getByRole('status')).toHaveTextContent('Check-in feito. Atendimento ATD-2026-118791, accession ACC-2026-052311.');
    expect(await within(feito).findByText('Ainda não chegaram do aparelho.')).toBeInTheDocument();

    expect(estado.pedidosDeCheckin).toHaveLength(1);
    const pedido = estado.pedidosDeCheckin[0];
    expect(pedido?.csrf).toBe('csrf-ficticio-dos-testes');
    expect(pedido?.chave).toMatch(UUID);
    expect(pedido?.corpo).toEqual({
      agendamento_id: ANTONIO.id,
      confirmacao: { nome: 'Antônio Carlos Ribeiro', nascimento: '1954-06-15' },
    });
  });

  it('sem os dois identificadores, a tela pede o que falta e não envia', async () => {
    const usuario = userEvent.setup();
    renderizar(URL_DO_DIA);
    const detalhe = await abrirAntonio(usuario);
    await usuario.click(within(detalhe).getByRole('button', { name: 'Fazer check-in' }));
    const gaveta = screen.getByRole('complementary', { name: 'Check-in do paciente' });
    await usuario.click(within(gaveta).getByRole('button', { name: 'Conferir e fazer check-in' }));
    expect(within(gaveta).getByText('Digite o nome completo como está no documento.')).toBeInTheDocument();
    expect(within(gaveta).getByText('Digite a data de nascimento completa: dia, mês e ano.')).toBeInTheDocument();
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(estado.pedidosDeCheckin).toHaveLength(0);
  });

  it('nascimento que não confere: mostra a mensagem do servidor, mantém o que foi digitado e troca a chave no novo envio', async () => {
    const usuario = userEvent.setup();
    renderizar(URL_DO_DIA);
    const detalhe = await abrirAntonio(usuario);
    await usuario.click(within(detalhe).getByRole('button', { name: 'Fazer check-in' }));
    await preencherEConferir(usuario, 'Antônio Carlos Ribeiro', '15061955');
    await usuario.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Fazer check-in' }));

    const gaveta = screen.getByRole('complementary', { name: 'Check-in do paciente' });
    const alerta = await within(gaveta).findByRole('alert');
    expect(alerta).toHaveTextContent('O nome ou o nascimento informado não confere com o cadastro. Nada foi gravado.');
    expect(alerta).toHaveTextContent('Código de suporte: trace-identificacao-nao-confere');
    expect(within(gaveta).getByRole('textbox', { name: 'Nome completo' })).toHaveValue('Antônio Carlos Ribeiro');
    expect(screen.queryByRole('complementary', { name: 'Check-in feito' })).not.toBeInTheDocument();

    await preencherEConferir(usuario, 'Antônio Carlos Ribeiro', '15061954');
    await usuario.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Fazer check-in' }));
    expect(await screen.findByRole('complementary', { name: 'Check-in feito' })).toBeInTheDocument();
    const [primeiro, segundo] = estado.pedidosDeCheckin;
    expect(primeiro?.chave).not.toBe(segundo?.chave);
  });

  it('rede caiu: diz que não se sabe se gravou e reenvia com a MESMA chave', async () => {
    const usuario = userEvent.setup();
    let falhar = true;
    const chaves: (string | null)[] = [];
    servidor.use(
      http.post('*/v1/atendimentos', ({ request }) => {
        if (falhar) {
          chaves.push(request.headers.get('Idempotency-Key'));
          return HttpResponse.error();
        }
        return undefined;
      }),
    );
    renderizar(URL_DO_DIA);
    const detalhe = await abrirAntonio(usuario);
    await usuario.click(within(detalhe).getByRole('button', { name: 'Fazer check-in' }));
    await preencherEConferir(usuario, 'Antônio Carlos Ribeiro', '15061954');
    await usuario.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Fazer check-in' }));

    const gaveta = screen.getByRole('complementary', { name: 'Check-in do paciente' });
    const aviso = await within(gaveta).findByText(/Não foi possível falar com o servidor\./);
    expect(aviso.closest('[role="status"]')).toHaveTextContent('Não dá para saber se o check-in foi registrado.');
    expect(chaves).toHaveLength(3);
    expect(new Set(chaves).size).toBe(1);

    falhar = false;
    await usuario.click(within(gaveta).getByRole('button', { name: 'Tentar de novo' }));
    expect(await screen.findByRole('complementary', { name: 'Check-in feito' })).toBeInTheDocument();
    expect(estado.pedidosDeCheckin[0]?.chave).toBe(chaves[0]);
  });

  it('check-in já feito: mostra a mensagem do servidor', async () => {
    const usuario = userEvent.setup();
    servidor.use(
      http.post('*/v1/atendimentos', () =>
        problema(409, 'checkin-ja-realizado', 'O check-in deste agendamento já foi feito por Carla Mendes às 08:52. Nada foi gravado de novo.'),
      ),
    );
    renderizar(URL_DO_DIA);
    const detalhe = await abrirAntonio(usuario);
    await usuario.click(within(detalhe).getByRole('button', { name: 'Fazer check-in' }));
    await preencherEConferir(usuario, 'Antônio Carlos Ribeiro', '15061954');
    await usuario.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Fazer check-in' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('já foi feito por Carla Mendes às 08:52');
  });

  it('Voltar no diálogo não envia nada', async () => {
    const usuario = userEvent.setup();
    renderizar(URL_DO_DIA);
    const detalhe = await abrirAntonio(usuario);
    await usuario.click(within(detalhe).getByRole('button', { name: 'Fazer check-in' }));
    await preencherEConferir(usuario, 'Antônio Carlos Ribeiro', '15061954');
    await usuario.keyboard('{Escape}');
    await waitFor(() => {
      expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    });
    expect(estado.pedidosDeCheckin).toHaveLength(0);
  });

  it('agendamento com atendimento mostra o atendimento em vez do botão de check-in', async () => {
    const usuario = userEvent.setup();
    renderizar(URL_DO_DIA);
    await usuario.click(await screen.findByRole('button', { name: /^Sônia Maria Castro, nascimento 03\/02\/1962/ }));
    const detalhe = screen.getByRole('complementary', { name: 'Detalhe do agendamento' });
    expect(within(detalhe).queryByRole('button', { name: 'Fazer check-in' })).not.toBeInTheDocument();
    expect(await within(detalhe).findByText('ATD-2026-118791')).toBeInTheDocument();
  });
});
