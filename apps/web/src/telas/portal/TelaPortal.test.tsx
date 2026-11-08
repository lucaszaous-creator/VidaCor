import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http } from 'msw';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { PORTAL_LAUDO_DOPPLER, PORTAL_LAUDO_ECO } from '../../../testes/dados/fase1';
import { problema, reiniciarNucleo, servidor } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Portal do paciente (FC-LAU-03, FC-LAU-05, FC-ACE)', () => {
  it('lista os laudos publicados com a marca de retificação e a letra ajustável', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/portal');
    const lista = await screen.findByRole('region', { name: 'Laudos publicados' });
    expect(await within(lista).findByText('Retificado')).toBeInTheDocument();
    expect(within(lista).getByText('Doppler colorido de carótidas')).toBeInTheDocument();
    expect(within(lista).getByRole('link', { name: 'Ler o laudo de Ecocardiograma transtorácico' })).toHaveAttribute(
      'href',
      `/portal/laudos/${PORTAL_LAUDO_ECO.id}`,
    );
    expect(screen.getByText('Letra 100%')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Diminuir a letra' })).toBeDisabled();
    await usuario.click(screen.getByRole('button', { name: 'Aumentar a letra' }));
    expect(screen.getByText('Letra 115%')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('leitura do laudo retificado: versão vigente, aviso de correção e texto sem HTML', async () => {
    const { container } = renderizar(`/portal/laudos/${PORTAL_LAUDO_ECO.id}`);
    const laudo = await screen.findByRole('article', { name: 'Laudo de Ecocardiograma transtorácico' });
    expect(laudo).toHaveTextContent('Este laudo foi corrigido pelo médico.');
    expect(laudo).toHaveTextContent('Você está vendo a versão corrigida, que substitui a anterior.');
    expect(laudo).toHaveTextContent('<b>sem negrito</b>');
    expect(laudo.querySelector('pre b')).toBeNull();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('o paciente baixa o PDF do próprio laudo, sem detalhe técnico na confirmação', async () => {
    const usuario = userEvent.setup();
    const { container, salvarArquivo } = renderizar(`/portal/laudos/${PORTAL_LAUDO_ECO.id}`);
    await screen.findByRole('article', { name: 'Laudo de Ecocardiograma transtorácico' });
    await usuario.click(screen.getByRole('button', { name: 'Baixar PDF' }));
    const status = await screen.findByText(/PDF baixado: LAU-2026-000031-v2\.pdf\./);
    expect(status).not.toHaveTextContent('SHA-256');
    expect(salvarArquivo).toHaveBeenCalledTimes(1);
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('PDF recusado com a sessão encerrada pede para entrar no portal e nada é baixado', async () => {
    const usuario = userEvent.setup();
    servidor.use(http.get('*/v1/portal/laudos/:id/pdf', () => problema(401, 'sem-sessao', 'Entre no portal.')));
    const { salvarArquivo } = renderizar(`/portal/laudos/${PORTAL_LAUDO_ECO.id}`);
    await usuario.click(await screen.findByRole('button', { name: 'Baixar PDF' }));
    expect(await screen.findByText(/Entre no portal para ver seus resultados\./)).toBeInTheDocument();
    expect(salvarArquivo).not.toHaveBeenCalled();
  });

  it('link compartilhado não oferece o PDF (só o paciente, na sessão dele)', async () => {
    renderizar('/portal/link/link-valido');
    await screen.findByRole('article', { name: 'Laudo de Doppler colorido de carótidas' });
    expect(screen.queryByRole('button', { name: 'Baixar PDF' })).not.toBeInTheDocument();
  });

  it('laudo de outro paciente e laudo inexistente recebem a mesma mensagem neutra (404)', async () => {
    const primeira = renderizar('/portal/laudos/0000001b-0000-7000-8000-000000000099');
    expect(await screen.findByText(/Laudo não encontrado\./)).toBeInTheDocument();
    const textoDaPrimeira = screen.getByText(/Laudo não encontrado\./).closest('[role="status"]')?.textContent;
    primeira.unmount();

    servidor.use(
      http.get('*/v1/portal/laudos/:id', () => problema(404, 'nao-encontrado', 'Não encontramos este laudo na sua conta. Confira a lista de resultados.')),
    );
    renderizar(`/portal/laudos/${PORTAL_LAUDO_DOPPLER.id}`);
    const segunda = await screen.findByText(/Laudo não encontrado\./);
    expect(segunda.closest('[role="status"]')?.textContent).toBe(textoDaPrimeira);
  });

  it('sem sessão do paciente, pede para entrar e não mostra nada', async () => {
    servidor.use(http.get('*/v1/portal/laudos', () => problema(401, 'sem-sessao', 'Entre no portal.')));
    renderizar('/portal');
    expect(await screen.findByText(/Entre no portal para ver seus resultados\./)).toBeInTheDocument();
  });

  it('link compartilhado expirado diz que expirou e não mostra o laudo (410)', async () => {
    renderizar('/portal/link/token-expirado-ontem');
    const aviso = await screen.findByText(/Este link expirou\./);
    expect(aviso.closest('[role="status"]')).toHaveTextContent('Peça ao paciente que compartilhe um link novo pelo portal.');
    expect(screen.queryByRole('article')).not.toBeInTheDocument();
  });

  it('link compartilhado válido mostra só o laudo compartilhado', async () => {
    renderizar('/portal/link/link-valido');
    expect(await screen.findByRole('article', { name: 'Laudo de Doppler colorido de carótidas' })).toHaveTextContent(
      'Placa menor que 50% na carótida esquerda.',
    );
  });
});
