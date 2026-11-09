import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor, UUID } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { SESSAO_COORDENACAO_MEDICA } from '../../../testes/nucleo-modelos';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.medicoSemVinculo;
});

describe('Revisão por pares (Revisao-Pares.dc.html)', () => {
  it('o revisor abre o caso com os dois identificadores, autor oculto, e envia o parecer confirmando', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/revisao-por-pares');
    expect(await screen.findByText('Minha fila, 1 de 3 feitas')).toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: /TC de tórax sem contraste/ }));
    const caso = screen.getByRole('region', { name: 'Caso em revisão' });
    expect(await within(caso).findByText('Antônio Carlos Ribeiro, nascido(a) em 15/06/1954')).toBeInTheDocument();
    expect(within(caso).getByText('Oculto até o envio')).toBeInTheDocument();
    expect(within(caso).getByRole('region', { name: 'Laudo original (versão assinada, somente leitura)' })).toHaveTextContent('Nódulo sólido de 6 mm');
    expect(within(caso).getByRole('button', { name: 'Enviar revisão' })).toBeDisabled();
    await usuario.click(within(within(caso).getByRole('radiogroup', { name: 'Concordância (RADPEER)' })).getByRole('radio', { name: '3a' }));
    expect(within(caso).getByText('Discrepância significativa (3a).')).toBeInTheDocument();
    await usuario.click(within(caso).getByRole('button', { name: 'Enviar revisão' }));
    expect(within(caso).getByText('Na discrepância, explique o que foi visto (10 letras ou mais).')).toBeInTheDocument();
    expect(pedidosDe('parecer-de-revisao')).toHaveLength(0);
    await usuario.type(within(caso).getByRole('textbox', { name: 'Comentário educativo para o colega' }), 'Linfonodo hilar de 12 mm não descrito.');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(caso).getByRole('button', { name: 'Enviar revisão' }));
    const dialogo = await screen.findByRole('alertdialog', { name: 'Enviar a revisão do laudo de Antônio Carlos Ribeiro com classificação 3a?' });
    expect(dialogo).toHaveTextContent('Antônio Carlos Ribeiro');
    expect(dialogo).toHaveTextContent('15/06/1954');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Enviar revisão' }));
    expect(await within(caso).findByText('Revisão REV-2026-001101 enviada com classificação 3a.')).toBeInTheDocument();
    expect(within(caso).getByText(/Autor revelado: Dra\. Cecília Souto\./)).toBeInTheDocument();
    const pedido = pedidosDe('parecer-de-revisao')[0];
    expect(pedido?.chave).toMatch(UUID);
    expect(pedido?.corpo).toEqual({ classe: '3a', comentario: 'Linfonodo hilar de 12 mm não descrito.' });
  });

  it('o autor vê a revisão sem saber quem revisou e decide a pendência', async () => {
    estado.sessao = SESSOES.medico;
    const usuario = userEvent.setup();
    renderizar('/revisao-por-pares');
    await usuario.click(await screen.findByRole('tab', { name: 'Revisões dos meus laudos' }));
    const lista = await screen.findByRole('list', { name: 'Revisões dos meus laudos' });
    expect(within(lista).getByText('Consolidação basal discreta à direita, não descrita.')).toBeInTheDocument();
    expect(within(lista).getByText('Aguarda o autor')).toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: 'Resultados' })).not.toBeInTheDocument();
    await usuario.click(within(lista).getByRole('button', { name: 'Já retifiquei' }));
    await esperarAlertaDoServidor('O laudo ainda não tem versão nova.', 'retificacao-nao-encontrada');
    await usuario.click(within(lista).getByRole('button', { name: 'Manter o laudo' }));
    expect(within(lista).getByText('Para manter, explique por quê (5 letras ou mais).')).toBeInTheDocument();
    await usuario.type(within(lista).getByRole('textbox', { name: 'Motivo para LAU-2026-003801' }), 'Achado sem relevância clínica, revisto com a imagem.');
    await usuario.click(within(lista).getByRole('button', { name: 'Manter o laudo' }));
    expect(await within(lista).findByText('Laudo mantido')).toBeInTheDocument();
    expect(pedidosDe('pendencia-de-revisao')[1]?.corpo).toEqual({ decisao: 'mantido', motivo: 'Achado sem relevância clínica, revisto com a imagem.' });
  });

  it('a coordenação médica sorteia e vê o resultado por médico, sem texto de laudo', async () => {
    estado.sessao = SESSAO_COORDENACAO_MEDICA;
    const usuario = userEvent.setup();
    const { container } = renderizar('/revisao-por-pares');
    await usuario.click(await screen.findByRole('tab', { name: 'Resultados' }));
    const tabela = await screen.findByRole('grid', { name: 'Resultado por médico, últimos 90 dias' });
    expect(within(tabela).getByText('Dr. Felipe Andrade')).toBeInTheDocument();
    expect(within(tabela).getByText('1, 1 aguardando o autor')).toBeInTheDocument();
    expect(screen.getByText('Confidencial: só o próprio médico e a coordenação médica veem o resultado individual.')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('button', { name: 'Sortear' }));
    expect(await screen.findByText('1 laudo(s) designado(s) para revisão.')).toBeInTheDocument();
    const sorteio = pedidosDe('sortear-revisoes')[0];
    expect(sorteio?.chave).toMatch(UUID);
    expect(sorteio?.corpo).toEqual({ quantidade: 6, dias: 30 });
  });

  it('caixa não abre a revisão por pares', async () => {
    estado.sessao = SESSOES.caixa;
    renderizar('/revisao-por-pares');
    expect(await screen.findByText('Seu perfil não revisa laudos')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /TC de tórax/ })).not.toBeInTheDocument();
  });
});
