import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { LAUDO_ANTONIO_ID, SESSOES } from '../../../testes/dados/fase1';
import { fase1 } from '../../../testes/nucleo-fase1';
import { SHA_FICTICIO } from '../../../testes/nucleo-documentos';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

const URL = `/laudos/${LAUDO_ANTONIO_ID}`;

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.medico;
});

function assinada(numero: number, texto: string, motivo: string | null) {
  return {
    numero,
    texto,
    situacao: 'assinada' as const,
    motivo,
    assinada_por: 'Dr. Paulo Medeiros',
    assinada_em: '2026-11-06T12:04:00Z',
    hash_documento: 'a'.repeat(64),
  };
}

function laudoAssinado(): void {
  fase1.laudo = {
    ...fase1.laudo,
    situacao: 'assinado',
    publicado_no_portal: true,
    versao_vigente: 1,
    versoes: [assinada(1, 'Ecocardiograma dentro da normalidade.', null)],
  };
}

describe('Laudo impresso em PDF', () => {
  it('laudo assinado: baixa o PDF e mostra o arquivo, a emissão e o SHA-256 dados pelo servidor', async () => {
    laudoAssinado();
    const usuario = userEvent.setup();
    const { container, salvarArquivo } = renderizar(URL);
    await usuario.click(await screen.findByRole('button', { name: 'Baixar PDF' }));
    const status = await screen.findByText(/PDF baixado: LAU-2026-004120-v1\.pdf\./);
    expect(status).toHaveTextContent('Emissão DOC-2026-000007 registrada');
    expect(status).toHaveTextContent(SHA_FICTICIO);
    expect(salvarArquivo).toHaveBeenCalledTimes(1);
    const [doc] = salvarArquivo.mock.calls[0] as [{ nomeDoArquivo: string; conteudo: Blob }];
    expect(doc.nomeDoArquivo).toBe('LAU-2026-004120-v1.pdf');
    expect(doc.conteudo.size).toBeGreaterThan(0);
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('rascunho sem versão assinada não oferece o PDF', async () => {
    renderizar(URL);
    expect(await screen.findByRole('textbox', { name: 'Texto do laudo, versão 1' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Baixar PDF' })).not.toBeInTheDocument();
  });

  it('com retificação em rascunho, o PDF sai da versão assinada que ainda vale', async () => {
    fase1.laudo = {
      ...fase1.laudo,
      situacao: 'retificacao_em_rascunho',
      versao_vigente: 1,
      versoes: [assinada(1, 'Fração de ejeção de 66%.', null), { ...assinada(2, 'Fração de ejeção de 64%.', 'Medida corrigida.'), situacao: 'rascunho', assinada_em: null, assinada_por: null, hash_documento: null }],
    };
    const usuario = userEvent.setup();
    renderizar(URL);
    const vigente = await screen.findByRole('region', { name: 'Versão 1 assinada (vigente)' });
    expect(vigente).toHaveTextContent('a versão 1 continua valendo e é a que sai no PDF');
    await usuario.click(within(vigente).getByRole('button', { name: 'Baixar PDF' }));
    expect(await within(vigente).findByText(/PDF baixado: LAU-2026-004120-v1\.pdf\./)).toBeInTheDocument();
  });

  it('servidor recusa (sem vínculo): a mensagem do servidor aparece e nada é baixado', async () => {
    laudoAssinado();
    const usuario = userEvent.setup();
    const { salvarArquivo } = renderizar(URL);
    const botao = await screen.findByRole('button', { name: 'Baixar PDF' });
    estado.sessao = SESSOES.medicoSemVinculo;
    await usuario.click(botao);
    expect(await screen.findByText(/Você não tem vínculo de cuidado com este paciente\./)).toBeInTheDocument();
    expect(salvarArquivo).not.toHaveBeenCalled();
  });
});
