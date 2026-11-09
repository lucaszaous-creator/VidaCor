import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor, UUID } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { LAUDO_ANTONIO_ID, SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { SESSAO_COORDENACAO_MEDICA } from '../../../testes/nucleo-modelos';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.medico;
});

describe('Modelos de laudo (Modelos-Laudo.dc.html)', () => {
  it('médico edita o modelo pessoal: nova versão com motivo, publicada na hora', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/modelos-de-laudo');
    await usuario.click(await screen.findByRole('button', { name: /Eco, Dr\. Paulo/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do modelo' });
    expect(within(detalhe).getByText('Modelo pessoal')).toBeInTheDocument();
    expect(within(detalhe).getByText(/Versão 2 de .*, usada em 845 laudos\./)).toBeInTheDocument();
    const texto = within(detalhe).getByRole('textbox', { name: 'Texto do modelo, versão 2' });
    await usuario.type(texto, ' Ritmo [ritmo].');
    await usuario.click(within(detalhe).getByRole('button', { name: 'Publicar versão 3' }));
    expect(within(detalhe).getByText('Escreva o motivo (5 letras ou mais).')).toBeInTheDocument();
    expect(pedidosDe('nova-versao-de-modelo')).toHaveLength(0);
    await usuario.type(within(detalhe).getByRole('textbox', { name: 'O que mudou e por quê' }), 'Inclui o ritmo');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(detalhe).getByRole('button', { name: 'Publicar versão 3' }));
    expect(await within(detalhe).findByText('Versão 3 publicada.')).toBeInTheDocument();
    const pedido = pedidosDe('nova-versao-de-modelo')[0];
    expect(pedido?.chave).toMatch(UUID);
    expect(pedido?.corpo).toMatchObject({ motivo: 'Inclui o ritmo', versao_base: 2 });
  });

  it('modelo da clínica com proposta: o médico só lê; a coordenação médica aprova', async () => {
    const usuario = userEvent.setup();
    renderizar('/modelos-de-laudo');
    await usuario.click(await screen.findByRole('button', { name: /TC de crânio sem contraste/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do modelo' });
    expect(within(detalhe).getByText('Versão 6 aguarda a coordenação médica.')).toBeInTheDocument();
    expect(within(detalhe).queryByRole('button', { name: 'Aprovar versão 6' })).not.toBeInTheDocument();
    expect(within(detalhe).getByText('Há uma proposta em aberto: aguarde a decisão para propor outra.')).toBeInTheDocument();
  });

  it('a coordenação médica aprova a proposta de outro médico', async () => {
    estado.sessao = SESSAO_COORDENACAO_MEDICA;
    const usuario = userEvent.setup();
    const { container } = renderizar('/modelos-de-laudo');
    expect(await screen.findByText('Coordenação médica: 1 proposta(s) aguardando sua decisão.')).toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: /TC de crânio sem contraste/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do modelo' });
    await usuario.click(within(detalhe).getByRole('button', { name: 'Recusar' }));
    expect(within(detalhe).getByText('Para recusar, escreva o motivo (5 letras ou mais).')).toBeInTheDocument();
    expect(pedidosDe('decidir-versao-de-modelo')).toHaveLength(0);
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(detalhe).getByRole('button', { name: 'Aprovar versão 6' }));
    expect(await within(screen.getByRole('region', { name: 'Detalhe do modelo' })).findByText(/Versão 6 de .*, usada em 1102 laudos\./)).toBeInTheDocument();
    expect(pedidosDe('decidir-versao-de-modelo')[0]?.corpo).toEqual({ aprovar: true, motivo: null });
  });

  it('modelo novo com dado de paciente é recusado pelo servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/modelos-de-laudo');
    await usuario.click(await screen.findByRole('button', { name: 'Novo modelo' }));
    const form = screen.getByRole('form', { name: 'Novo modelo' });
    await usuario.type(within(form).getByRole('textbox', { name: 'Nome do modelo' }), 'Doppler de carótidas');
    await usuario.type(within(form).getByRole('textbox', { name: 'Exame' }), 'Doppler de carótidas e vertebrais');
    await usuario.type(within(form).getByRole('textbox', { name: 'Modalidade' }), 'us');
    await usuario.type(within(form).getByRole('textbox', { name: 'Texto do modelo' }), 'Paciente nascida em 12/04/1993.');
    await usuario.click(within(form).getByRole('button', { name: 'Criar e publicar' }));
    await esperarAlertaDoServidor('Modelo e frase não levam dado de paciente', 'modelo-com-dado-de-paciente');
    expect(pedidosDe('criar-modelo')[0]?.corpo).toMatchObject({ tipo: 'pessoal', modalidade: 'US' });
  });

  it('frases rápidas: lista, testa o atalho e cria frase pessoal', async () => {
    const usuario = userEvent.setup();
    renderizar('/modelos-de-laudo');
    await usuario.click(await screen.findByRole('tab', { name: 'Frases rápidas' }));
    const tabela = await screen.findByRole('grid', { name: 'Frases rápidas' });
    expect(within(tabela).getByText('.retorno')).toBeInTheDocument();
    await usuario.type(screen.getByRole('searchbox', { name: 'Experimente um atalho' }), '.normal');
    expect(screen.getByText('Exame dentro dos limites da normalidade para a idade e o sexo do paciente.', { selector: '[role="status"]' })).toBeInTheDocument();
    expect(screen.queryByRole('checkbox', { name: /Frase da clínica/ })).not.toBeInTheDocument();
    const form = screen.getByRole('form', { name: 'Nova frase' });
    await usuario.type(within(form).getByRole('textbox', { name: 'Atalho' }), 'limita');
    await usuario.type(within(form).getByRole('textbox', { name: 'Vira o texto' }), 'O método tem limitações.');
    await usuario.click(within(form).getByRole('button', { name: 'Criar frase' }));
    expect(await within(form).findByText('Frase .limita criada.')).toBeInTheDocument();
    expect(pedidosDe('criar-frase')[0]?.corpo).toEqual({ tipo: 'pessoal', atalho: '.limita', texto: 'O método tem limitações.' });
  });

  it('recepção não usa modelos de laudo', async () => {
    estado.sessao = SESSOES.recepcao;
    renderizar('/modelos-de-laudo');
    expect(await screen.findByText('Seu perfil não usa modelos de laudo')).toBeInTheDocument();
  });

  it('no rascunho do laudo, insere o modelo e uma frase pelo atalho', async () => {
    const usuario = userEvent.setup();
    renderizar(`/laudos/${LAUDO_ANTONIO_ID}`);
    const texto = await screen.findByRole('textbox', { name: /Texto do laudo, versão/ });
    await usuario.click(screen.getByText('Modelos e frases'));
    const escolha = await screen.findByRole('combobox', { name: 'Modelo' });
    await screen.findByRole('option', { name: /Eco, Dr\. Paulo/ });
    await usuario.selectOptions(escolha, screen.getByRole('option', { name: /Eco, Dr\. Paulo/ }));
    await usuario.click(screen.getByRole('button', { name: 'Inserir modelo' }));
    expect(await screen.findByText('Modelo MOD-2026-000002 v2 inserido no rascunho.')).toBeInTheDocument();
    expect((texto as HTMLTextAreaElement).value).toContain('Frase pessoal de conclusão.');
    const uso = pedidosDe('usar-modelo')[0];
    expect(uso?.chave).toMatch(UUID);
    expect(uso?.corpo).toEqual({ modelo_id: 'mod-eco-paulo', numero: 2 });
    await usuario.type(screen.getByRole('textbox', { name: 'Atalho da frase' }), '.retorno');
    await usuario.click(screen.getByRole('button', { name: 'Inserir frase' }));
    expect((texto as HTMLTextAreaElement).value).toContain('Sugere-se controle ecográfico conforme critério do médico assistente.');
  });
});
