import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor, UUID } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { PULSEIRAS, SALA_1, SALA_6, SESSOES, TRIAGEM_PRECISA_APROVACAO } from '../../../testes/dados/fase1';
import { fase1, pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, problema, reiniciarNucleo, servidor } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.tecnico;
});

async function bipar(codigo: string): Promise<void> {
  const usuario = userEvent.setup();
  const campo = await screen.findByRole('textbox', { name: /Código da pulseira/ });
  await usuario.type(campo, `${codigo}{Enter}`);
}

describe('Sala de exame: bipe da pulseira (FC-PUL)', () => {
  it('pulseira certa libera a worklist, mostra o paciente em foco e o leitor (teclado + Enter) fica pronto', async () => {
    const { container } = renderizar(`/sala?sala=${SALA_1.id}`);
    expect(await screen.findByRole('heading', { name: 'Sala 1, eco' })).toBeInTheDocument();
    const campo = screen.getByRole('textbox', { name: /Código da pulseira/ });
    expect(campo).toHaveFocus();
    expect(await screen.findByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954, 09:00/ })).toBeInTheDocument();
    expect(screen.getByText('Nenhum paciente em foco.')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Sala de exame/ })).toHaveAttribute('aria-current', 'page');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);

    await bipar(PULSEIRAS.antonio);
    const feito = await screen.findByText(/Exame liberado\./);
    expect(feito.closest('[role="status"]')).toHaveTextContent(
      'Lista de trabalho enviada ao Vivid E95 (VIVIDE95) com Antônio Carlos Ribeiro, nascimento 15/06/1954.',
    );
    expect(screen.getByRole('region', { name: 'Paciente em foco: Antônio Carlos Ribeiro, nascimento 15/06/1954' })).toBeInTheDocument();
    expect(campo).toHaveValue('');
    expect(campo).toHaveFocus();
    expect(pedidosDe('bipe')[0]?.corpo).toEqual({ codigo_etiqueta: PULSEIRAS.antonio });
    expect(pedidosDe('bipe')[0]?.chave).toMatch(UUID);

    await bipar(PULSEIRAS.antonio);
    expect(await screen.findByText(/A lista de trabalho já estava no aparelho\./)).toBeInTheDocument();
    expect(pedidosDe('bipe')[1]?.chave).not.toBe(pedidosDe('bipe')[0]?.chave);
  });

  it('pulseira de outra sala bloqueia, diz a sala certa e oferece a fila dela', async () => {
    renderizar(`/sala?sala=${SALA_1.id}`);
    await bipar(PULSEIRAS.juliana);
    const alerta = await esperarAlertaDoServidor('A pulseira é de Juliana Sampaio Rocha, 12/04/1993, agendada para a Sala 2.', 'paciente-de-outra-sala');
    expect(alerta).toHaveTextContent('Paciente da Sala 2, obstétrica: encaminhe e bipe a próxima pulseira.');
    expect(within(alerta).getByRole('link', { name: 'Ver a fila da Sala 2, obstétrica' })).toBeInTheDocument();
    expect(screen.getByText('Nenhum paciente em foco.')).toBeInTheDocument();
  });

  it.each([
    ['etiqueta-substituida', PULSEIRAS.invalidada, 'foi substituída por reimpressão. O exame não foi liberado'],
    ['etiqueta-nao-encontrada', 'ETQ-2026-999999', 'não foi encontrada. Nada foi enviado ao aparelho.'],
  ])('%s: bloqueia com a mensagem do servidor', async (codigo, pulseira, trecho) => {
    renderizar(`/sala?sala=${SALA_1.id}`);
    await bipar(pulseira);
    await esperarAlertaDoServidor(trecho, codigo);
  });

  it('exame com contraste sem triagem não inicia (422 triagem-ausente)', async () => {
    renderizar(`/sala?sala=${SALA_6.id}`);
    expect(await screen.findByRole('link', { name: 'Triagem de Vera Lúcia Antunes' })).toBeInTheDocument();
    expect(screen.getByText('Contraste: sem triagem')).toBeInTheDocument();
    await bipar(PULSEIRAS.vera);
    await esperarAlertaDoServidor('O exame com contraste não tem triagem registrada.', 'triagem-ausente');
  });

  it('triagem pedindo aprovação médica sem conduta bloqueia (409 conduta-medica-pendente)', async () => {
    fase1.triagemVera = { ...TRIAGEM_PRECISA_APROVACAO };
    renderizar(`/sala?sala=${SALA_6.id}`);
    expect(await screen.findByText('Contraste: triagem aguarda a conduta médica')).toBeInTheDocument();
    await bipar(PULSEIRAS.vera);
    await esperarAlertaDoServidor('a conduta ainda não foi registrada', 'conduta-medica-pendente');
  });

  it('técnico de outra sala recebe 403 com a mensagem do servidor', async () => {
    servidor.use(
      http.post('*/v1/salas/:id/bipes', () =>
        problema(403, 'sem-permissao', 'Você está lotado na Sala 6 e não libera exames da Sala 1. Nada foi enviado ao aparelho.'),
      ),
    );
    renderizar(`/sala?sala=${SALA_1.id}`);
    await bipar(PULSEIRAS.antonio);
    await esperarAlertaDoServidor('Você está lotado na Sala 6 e não libera exames da Sala 1.', 'sem-permissao');
  });

  it('rede caiu: reenvia o mesmo bipe com a MESMA chave', async () => {
    let falhar = true;
    servidor.use(
      http.post('*/v1/salas/:id/bipes', ({ request }) => {
        if (falhar) {
          fase1.pedidos.push({ rota: 'bipe-falho', chave: request.headers.get('Idempotency-Key'), csrf: null, estacao: null, corpo: null });
          return HttpResponse.error();
        }
        return undefined;
      }),
    );
    const usuario = userEvent.setup();
    renderizar(`/sala?sala=${SALA_1.id}`);
    await bipar(PULSEIRAS.antonio);
    expect(await screen.findByText(/Não foi possível falar com o servidor\./)).toBeInTheDocument();
    falhar = false;
    await usuario.click(screen.getByRole('button', { name: 'Tentar de novo' }));
    expect(await screen.findByText(/Exame liberado\./)).toBeInTheDocument();
    const falhos = pedidosDe('bipe-falho').map((p) => p.chave);
    expect(new Set(falhos).size).toBe(1);
    expect(pedidosDe('bipe')[0]?.chave).toBe(falhos[0]);
  });

  it('fila vazia e erro de carga aparecem como estados, nunca como lista vazia sem explicação', async () => {
    servidor.use(http.get('*/v1/salas/:id/fila', () => problema(500, 'erro-interno', 'A fila não pôde ser lida agora. Nada foi alterado.')));
    renderizar(`/sala?sala=${SALA_1.id}`);
    await waitFor(async () => {
      await esperarAlertaDoServidor('A fila não pôde ser lida agora.', 'erro-interno');
    });
  });
});
