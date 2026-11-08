import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http } from 'msw';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData, esperarAlertaDoServidor, UUID, voltarDa } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { ATENDIMENTO_VERA, SESSOES, TRIAGEM_PRECISA_APROVACAO } from '../../../testes/dados/fase1';
import { fase1, pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, problema, reiniciarNucleo, servidor } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

const URL = `/triagem/${ATENDIMENTO_VERA.id}`;

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.enfermagem;
});

describe('Triagem com contraste (FC-TRI)', () => {
  it('a tela pergunta e mostra o que o servidor calculou: resultado, TFG, motivos e versão da regra', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar(URL);
    expect(await screen.findByRole('region', { name: 'Paciente em foco: Vera Lúcia Antunes, nascimento 18/09/1957' })).toBeInTheDocument();
    for (const [pergunta, resposta] of [
      ['Já teve reação a contraste iodado?', 'Não'],
      ['Existe possibilidade de gravidez?', 'Não'],
      ['Está amamentando?', 'Não'],
      ['Usa metformina?', 'Sim'],
    ] as const) {
      await usuario.click(within(screen.getByRole('radiogroup', { name: pergunta })).getByRole('radio', { name: resposta }));
    }
    await usuario.type(screen.getByRole('textbox', { name: 'Creatinina (mg/dL)' }), '1,4');
    await digitarData(usuario, 'Data do exame de creatinina', '28102026');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('button', { name: 'Salvar triagem' }));

    const resultado = screen.getByRole('region', { name: 'Resultado da triagem' });
    expect(await within(resultado).findByText('Precisa de aprovação médica')).toBeInTheDocument();
    expect(resultado).toHaveTextContent('40 mL/min/1,73 m², calculada pelo servidor (CKD-EPI 2021)');
    expect(resultado).toHaveTextContent('TFG 40 mL/min/1,73 m², abaixo de 45');
    expect(resultado).toHaveTextContent('TRI-2026.1');
    expect(resultado).toHaveTextContent('Exame bloqueado até a conduta médica.');
    expect(within(resultado).queryByRole('textbox', { name: 'Conduta' })).not.toBeInTheDocument();

    const pedido = pedidosDe('triagem')[0];
    expect(pedido?.chave).toMatch(UUID);
    expect(pedido?.corpo).toEqual({
      reacao_previa_contraste: false,
      gestante: false,
      amamentando: false,
      usa_metformina: true,
      creatinina_mg_dl: '1.4',
      creatinina_data: '2026-10-28',
    });
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('sem respostas a tela pede cada uma e não envia; nada vem pré-marcado', async () => {
    const usuario = userEvent.setup();
    renderizar(URL);
    await screen.findByRole('radiogroup', { name: 'Usa metformina?' });
    expect(screen.getAllByRole('radio').filter((r) => (r as HTMLInputElement).checked)).toHaveLength(0);
    await usuario.click(screen.getByRole('button', { name: 'Salvar triagem' }));
    expect(screen.getAllByText('Responda sim ou não.')).toHaveLength(4);
    expect(screen.getByText('Digite a creatinina com até duas casas, ex.: 1,4.')).toBeInTheDocument();
    expect(pedidosDe('triagem')).toHaveLength(0);
  });

  it('médica registra a conduta com reautenticação e volta com a MESMA chave', async () => {
    fase1.triagemVera = { ...TRIAGEM_PRECISA_APROVACAO };
    fase1.exigirReautenticacao.add('conduta');
    estado.sessao = SESSOES.medicaTc;
    const usuario = userEvent.setup();
    const primeira = renderizar(URL);
    const conduta = await screen.findByRole('textbox', { name: 'Conduta' });
    await usuario.type(conduta, 'prosseguir com hidratação');
    await usuario.click(screen.getByRole('button', { name: 'Registrar conduta e liberar' }));
    await screen.findByText(/Confirme sua senha e o segundo fator para registrar a conduta\./);
    await usuario.click(screen.getByRole('button', { name: 'Confirmar identidade' }));
    const voltar = voltarDa(String(primeira.irPara.mock.calls[0]?.[0]));
    const chave = pedidosDe('conduta')[0]?.chave ?? '';
    expect(voltar).toBe(`${URL}?retomar=conduta&chave=${chave}`);
    primeira.unmount();

    fase1.exigirReautenticacao.delete('conduta');
    renderizar(voltar);
    expect(await screen.findByText(/Identidade confirmada\./)).toBeInTheDocument();
    await usuario.type(screen.getByRole('textbox', { name: 'Conduta' }), 'prosseguir com hidratação');
    await usuario.click(screen.getByRole('button', { name: 'Registrar conduta e liberar' }));
    expect(await screen.findByText(/Conduta registrada\./)).toBeInTheDocument();
    expect(pedidosDe('conduta')[1]?.chave).toBe(chave);
    expect(screen.getByRole('region', { name: 'Resultado da triagem' })).toHaveTextContent('prosseguir com hidratação (registrada em 06/11/2026 08:58)');
  });

  it('médico sem vínculo recebe 403 e a tela mostra a mensagem do servidor', async () => {
    fase1.triagemVera = { ...TRIAGEM_PRECISA_APROVACAO };
    estado.sessao = SESSOES.medico;
    servidor.use(
      http.post('*/v1/atendimentos/:id/triagem/conduta', () =>
        problema(403, 'sem-vinculo-de-cuidado', 'Só a médica com vínculo de cuidado registra a conduta. Nada foi gravado.'),
      ),
    );
    const usuario = userEvent.setup();
    renderizar(URL);
    await usuario.type(await screen.findByRole('textbox', { name: 'Conduta' }), 'prosseguir');
    await usuario.click(screen.getByRole('button', { name: 'Registrar conduta e liberar' }));
    await esperarAlertaDoServidor('Só a médica com vínculo de cuidado registra a conduta.', 'sem-vinculo-de-cuidado');
  });

  it('cadastro sem sexo (422) impede o cálculo e a tela diz o que fazer', async () => {
    servidor.use(
      http.post('*/v1/atendimentos/:id/triagem', () =>
        problema(422, 'cadastro-sem-sexo', 'O cadastro da paciente não tem o sexo, necessário para calcular a TFG. Nada foi gravado. Peça à recepção para completar o cadastro.'),
      ),
    );
    const usuario = userEvent.setup();
    renderizar(URL);
    for (const pergunta of ['Já teve reação a contraste iodado?', 'Existe possibilidade de gravidez?', 'Está amamentando?', 'Usa metformina?']) {
      await usuario.click(within(await screen.findByRole('radiogroup', { name: pergunta })).getByRole('radio', { name: 'Não' }));
    }
    await usuario.type(screen.getByRole('textbox', { name: 'Creatinina (mg/dL)' }), '0,9');
    await digitarData(usuario, 'Data do exame de creatinina', '28102026');
    await usuario.click(screen.getByRole('button', { name: 'Salvar triagem' }));
    await esperarAlertaDoServidor('O cadastro da paciente não tem o sexo', 'cadastro-sem-sexo');
  });

  it('técnico que tenta salvar triagem sem permissão vê a mensagem do servidor', async () => {
    estado.sessao = SESSOES.recepcao;
    const usuario = userEvent.setup();
    renderizar(URL);
    for (const pergunta of ['Já teve reação a contraste iodado?', 'Existe possibilidade de gravidez?', 'Está amamentando?', 'Usa metformina?']) {
      await usuario.click(within(await screen.findByRole('radiogroup', { name: pergunta })).getByRole('radio', { name: 'Não' }));
    }
    await usuario.type(screen.getByRole('textbox', { name: 'Creatinina (mg/dL)' }), '0,9');
    await digitarData(usuario, 'Data do exame de creatinina', '28102026');
    await usuario.click(screen.getByRole('button', { name: 'Salvar triagem' }));
    await esperarAlertaDoServidor('Seu perfil não pode registrar triagem.', 'sem-permissao');
  });
});
