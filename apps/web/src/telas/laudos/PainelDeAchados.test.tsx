import { screen, waitFor, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor, UUID } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { LAUDO_ANTONIO_ID, SESSOES } from '../../../testes/dados/fase1';
import { fase1, pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

const URL = `/laudos/${LAUDO_ANTONIO_ID}`;

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.medico;
});

async function painel(): Promise<HTMLElement> {
  return screen.findByRole('region', { name: 'Achados estruturados' });
}

async function escolher(usuario: UserEvent, dentro: HTMLElement, grupo: string, opcao: string): Promise<void> {
  await usuario.click(within(within(dentro).getByRole('radiogroup', { name: grupo })).getByRole('radio', { name: opcao }));
}

async function preencherNoduloTr4(usuario: UserEvent, p: HTMLElement): Promise<void> {
  await usuario.type(within(p).getByRole('textbox', { name: 'Localização' }), 'Lobo direito, terço médio');
  await usuario.type(within(p).getByRole('textbox', { name: 'Maior diâmetro (mm)' }), '18');
  await escolher(usuario, p, 'Composição', 'Sólido');
  await escolher(usuario, p, 'Ecogenicidade', 'Hipoecoico');
  await escolher(usuario, p, 'Forma', 'Mais largo que alto');
  await escolher(usuario, p, 'Margens', 'Lisas');
}

describe('Laudo estruturado: achados com categoria do servidor (docs/22 E2)', () => {
  it('registra um nódulo TI-RADS e mostra a categoria, a conduta e a versão da regra devolvidas pelo servidor', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar(URL);
    const p = await painel();
    expect(await within(p).findByText('Nenhum achado estruturado. Registre abaixo, se o exame tiver.')).toBeInTheDocument();
    await preencherNoduloTr4(usuario, p);
    await usuario.click(within(p).getByRole('button', { name: 'Registrar achado' }));

    expect(await within(p).findByText('N1: 4 pontos, TR4 moderadamente suspeito')).toBeInTheDocument();
    expect(within(p).getByText('Conduta: PAAF indicada (a partir de 15 mm).')).toBeInTheDocument();
    expect(within(p).getByText('Calculado pelo servidor com a tabela TIRADS-ACR-2017.1; a versão fica guardada no laudo.')).toBeInTheDocument();
    const tabela = within(p).getByRole('grid', { name: 'Achados do laudo' });
    expect(within(tabela).getByText('Lobo direito, terço médio, 18 mm')).toBeInTheDocument();

    const pedido = pedidosDe('achado')[0];
    expect(pedido?.chave).toMatch(UUID);
    // A tela manda só os descritores: nada de pontos, categoria ou conduta (G1).
    expect(pedido?.corpo).toEqual({
      sistema: 'tirads',
      localizacao: 'Lobo direito, terço médio',
      maior_diametro_mm: 18,
      tirads: { composicao: 'solida_ou_quase_solida', ecogenicidade: 'hipoecoica', forma: 'mais_largo_que_alto', margem: 'lisa', focos: [] },
    });
    // Formulário limpo para o próximo achado, com chave nova.
    expect(within(p).getByRole('textbox', { name: 'Localização' })).toHaveValue('');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('formulário incompleto ou medida fora do formato não chega ao servidor', async () => {
    const usuario = userEvent.setup();
    renderizar(URL);
    const p = await painel();
    await usuario.click(within(p).getByRole('button', { name: 'Registrar achado' }));
    expect(within(p).getByText('Informe a localização do achado.')).toBeInTheDocument();
    await usuario.type(within(p).getByRole('textbox', { name: 'Localização' }), 'Istmo');
    expect(within(p).getByText('Marque composição, ecogenicidade, forma e margens.')).toBeInTheDocument();
    await usuario.type(within(p).getByRole('textbox', { name: 'Maior diâmetro (mm)' }), 'dezoito');
    expect(within(p).getByText('Use números em milímetros, por exemplo 18 ou 8,5.')).toBeInTheDocument();
    expect(pedidosDe('achado')).toHaveLength(0);
  });

  it('BI-RADS recusado pelo servidor mostra a mensagem dele e mantém o que foi marcado', async () => {
    const usuario = userEvent.setup();
    renderizar(URL);
    const p = await painel();
    await escolher(usuario, p, 'Classificação', 'BI-RADS (mama)');
    await usuario.type(within(p).getByRole('textbox', { name: 'Localização' }), 'Quadrante superolateral, 2 horas');
    await usuario.type(within(p).getByRole('textbox', { name: 'Maior diâmetro (mm)' }), '14');
    await escolher(usuario, p, 'Lado', 'Mama esquerda');
    await escolher(usuario, p, 'Categoria BI-RADS', '6');
    await escolher(usuario, p, 'Forma', 'Irregular');
    await escolher(usuario, p, 'Orientação', 'Não paralela');
    await usuario.click(within(within(p).getByRole('group', { name: 'Margens' })).getByRole('checkbox', { name: 'Espiculada' }));
    await escolher(usuario, p, 'Padrão ecogênico', 'Hipoecoico');
    await escolher(usuario, p, 'Achados posteriores', 'Sombra');
    await usuario.click(within(p).getByRole('button', { name: 'Registrar achado' }));
    await esperarAlertaDoServidor('BI-RADS 6 exige malignidade confirmada por biópsia.', 'birads-6-sem-biopsia', p);
    expect(within(p).getByRole('textbox', { name: 'Localização' })).toHaveValue('Quadrante superolateral, 2 horas');
    expect(pedidosDe('achado')[0]?.corpo).toMatchObject({
      sistema: 'birads',
      birads: { lado: 'esquerda', categoria_atribuida: '6', biopsia_maligna_confirmada: false, massa: { margens: ['espiculada'] } },
    });
  });

  it('leva o texto gerado dos achados ao editor do rascunho e remove achado sem apagar', async () => {
    const usuario = userEvent.setup();
    renderizar(URL);
    const p = await painel();
    await preencherNoduloTr4(usuario, p);
    await usuario.click(within(p).getByRole('button', { name: 'Registrar achado' }));
    await usuario.click(await within(p).findByRole('button', { name: 'Levar os achados ao texto do laudo' }));
    const texto = screen.getByRole('textbox', { name: 'Texto do laudo, versão 1' });
    expect((texto as HTMLTextAreaElement).value).toContain('Conclusão:\nN1: TR4, moderadamente suspeito; PAAF indicada (a partir de 15 mm).');
    expect(within(p).getByText('Texto dos achados acrescentado ao laudo. Revise antes de assinar.')).toBeInTheDocument();

    await usuario.click(within(p).getByRole('button', { name: 'Remover N1 do rascunho' }));
    expect(await within(p).findByText('Nenhum achado estruturado. Registre abaixo, se o exame tiver.')).toBeInTheDocument();
    expect(pedidosDe('remocao-achado')).toHaveLength(1);
  });

  it('laudo assinado mostra os achados só para leitura, sem formulário', async () => {
    const v = fase1.laudo.versoes[0];
    if (v === undefined) throw new Error('sem versão');
    fase1.laudo = {
      ...fase1.laudo,
      situacao: 'assinado',
      versao_vigente: 1,
      publicado_no_portal: true,
      versoes: [{ ...v, situacao: 'assinada', assinada_em: '2026-11-06T12:04:00Z', hash_documento: 'sha256:11111111' }],
    };
    renderizar(URL);
    const p = await painel();
    expect(within(p).getByText('Somente leitura')).toBeInTheDocument();
    expect(within(p).queryByRole('button', { name: 'Registrar achado' })).not.toBeInTheDocument();
    expect(within(p).getByText('Este laudo não tem achados estruturados.')).toBeInTheDocument();
  });

  it('médico sem vínculo não vê os achados (o laudo nem abre)', async () => {
    estado.sessao = SESSOES.medicoSemVinculo;
    renderizar(URL);
    expect(await screen.findByText('Você não tem vínculo de cuidado com este paciente')).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.queryByRole('region', { name: 'Achados estruturados' })).not.toBeInTheDocument();
    });
  });
});
