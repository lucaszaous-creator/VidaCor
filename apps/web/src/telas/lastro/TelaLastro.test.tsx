import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http } from 'msw';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { SHA_FICTICIO } from '../../../testes/nucleo-documentos';
import { lastroSimulado } from '../../../testes/nucleo-estoque';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, problema, reiniciarNucleo, servidor } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

async function abrir(codigo: string) {
  const usuario = userEvent.setup();
  const r = renderizar('/lastro');
  await usuario.type(await screen.findByRole('searchbox', { name: 'Buscar por qualquer código' }), codigo);
  await usuario.click(screen.getByRole('button', { name: 'Abrir cadeia' }));
  return r;
}

describe('Explorador de lastro (Lastro.dc.html)', () => {
  it('abre a cadeia pelo código, com paciente, registros e eventos conferidos', async () => {
    const { container } = await abrir('ATD-2026-000031');
    expect(await screen.findByRole('heading', { name: 'Sônia Maria Castro' })).toBeInTheDocument();
    expect(screen.getByText(/Cadeia íntegra\./)).toBeInTheDocument();
    const eventos = screen.getByRole('region', { name: 'Eventos' });
    expect(within(eventos).getAllByText('Confere')).toHaveLength(2);
    expect(within(screen.getByRole('region', { name: 'Cadeia do atendimento' })).getByText('Etiqueta')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('evento que não confere aparece em destaque e a tela diz o que fazer', async () => {
    lastroSimulado.integra = false;
    await abrir('ATD-2026-000031');
    expect(await screen.findByText(/Há evento que não confere com o hash\./)).toBeInTheDocument();
    expect(screen.getByText('Não confere')).toBeInTheDocument();
  });

  it('emite o dossiê em PDF da cadeia aberta, com o CSRF, e mostra a emissão e o SHA-256', async () => {
    const usuario = userEvent.setup();
    const { container, salvarArquivo } = await abrir('ATD-2026-000031');
    const cadeia = screen.getByRole('region', { name: 'Cadeia do atendimento' });
    await usuario.click(await within(cadeia).findByRole('button', { name: 'Dossiê em PDF' }));
    const status = await within(cadeia).findByText(/PDF baixado: DOS-2026-000003\.pdf\./);
    expect(status).toHaveTextContent(`Emissão DOS-2026-000003 registrada, SHA-256 ${SHA_FICTICIO}.`);
    expect(salvarArquivo).toHaveBeenCalledTimes(1);
    const pedido = pedidosDe('dossie').at(-1);
    expect(pedido?.corpo).toEqual({ codigo: 'ATD-2026-000031' });
    expect(pedido?.csrf).toBe(estado.sessao.csrf);
    expect(pedido?.chave).toBeNull();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('dossiê recusado pelo servidor mostra a mensagem e nada é baixado', async () => {
    const usuario = userEvent.setup();
    servidor.use(http.post('*/v1/lastro/dossie', () => problema(403, 'sem-permissao', 'Seu perfil não pode emitir o dossiê. Nada foi feito.')));
    const { salvarArquivo } = await abrir('ATD-2026-000031');
    await usuario.click(await screen.findByRole('button', { name: 'Dossiê em PDF' }));
    expect(await screen.findByText(/Seu perfil não pode emitir o dossiê\./)).toBeInTheDocument();
    expect(salvarArquivo).not.toHaveBeenCalled();
  });

  it('código desconhecido mostra a mensagem do servidor; recepção não abre', async () => {
    await abrir('XYZ-1');
    expect(await screen.findByText(/Nenhum atendimento desta unidade tem este código\./)).toBeInTheDocument();
  });
});
