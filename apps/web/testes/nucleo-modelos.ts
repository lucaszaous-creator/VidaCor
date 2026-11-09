/**
 * Modelos de laudo, frases rápidas e revisão por pares simulados (Modelos-Laudo.dc.html e
 * Revisao-Pares.dc.html). Só imitam o contrato para a tela; as regras (quem propõe não aprova, autor
 * não revisa, um revisor por laudo, idempotência) são provadas em `crates/api/tests/modelos_laudo.rs`
 * e `crates/api/tests/revisao_pares.rs`, com banco real. Dados fictícios.
 */
import type {
  CasoDeRevisao,
  FraseDeLaudo,
  ItemDaFilaDeRevisao,
  ModeloDeLaudo,
  NovaVersaoDeModelo,
  NovoModeloDeLaudo,
  PainelDeRevisao,
  ParecerDeRevisao,
  RevisoesDoAutor,
  Sessao,
} from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { SESSOES } from './dados/fase1';
import { registrar, semPermissao, temPerfil } from './nucleo-fase1';
import { estado as nucleo } from './nucleo-simulado';
import { problema } from './problema';

/** Coordenação médica: médica com perfil de coordenação (DEC-2026-11-08-100). */
export const SESSAO_COORDENACAO_MEDICA: Sessao = {
  ...SESSOES.medicaTc,
  usuario: { ...SESSOES.medicaTc.usuario, perfis: ['medico', 'gerente'] },
};

const ehCoordenacao = (): boolean => temPerfil('medico') && temPerfil('gerente');

const TEXTO_ECO =
  'Técnica: exame realizado com aparelho [aparelho], em repouso, ritmo [ritmo].\nAchados: dimensões das cavidades cardíacas dentro da normalidade. Septo interventricular com [medida].\nConclusão: ecocardiograma [resumo].';
const TEXTO_TC = 'Técnica: tomografia sem contraste, cortes axiais.\nAchados: [achados].\nConclusão: [conclusao].';

const versao = (numero: number, texto: string, situacao: ModeloDeLaudo['versoes'][number]['situacao'], proposta_por: string, motivo: string | null, decidido_por: string | null = null) => ({
  numero,
  texto,
  motivo,
  situacao,
  proposta_por,
  decidido_por,
  decidido_em: decidido_por === null ? null : '2026-10-01T13:00:00Z',
  motivo_decisao: null,
  criado_em: '2026-10-01T12:00:00Z',
});

function modelosIniciais(): ModeloDeLaudo[] {
  return [
    {
      id: 'mod-eco', codigo: 'MOD-2026-000001', tipo: 'clinica', nome: 'Ecocardiograma transtorácico', exame: 'Ecocardiograma transtorácico', modalidade: 'US', autor: 'Clínica',
      ativo: true, motivo_desativacao: null, versao_publicada: 3, proposta_em_aberto: null, usos: 2077, pode_editar: true, pode_decidir: false, pode_desativar: false,
      versoes: [versao(3, TEXTO_ECO, 'publicada', 'Dr. Paulo Medeiros', 'Inclui frase de limitação do método', 'Dra. Cecília Souto'), versao(2, 'Texto anterior [x]', 'substituida', 'Dr. Paulo Medeiros', 'Campo de medida vem do aparelho', 'Dra. Cecília Souto'), versao(1, 'Primeiro texto [x]', 'substituida', 'Dra. Cecília Souto', null)],
    },
    {
      id: 'mod-eco-paulo', codigo: 'MOD-2026-000002', tipo: 'pessoal', nome: 'Eco, Dr. Paulo', exame: 'Ecocardiograma transtorácico', modalidade: 'US', autor: 'Dr. Paulo Medeiros',
      ativo: true, motivo_desativacao: null, versao_publicada: 2, proposta_em_aberto: null, usos: 845, pode_editar: true, pode_decidir: false, pode_desativar: true,
      versoes: [versao(2, `${TEXTO_ECO}\nFrase pessoal de conclusão.`, 'publicada', 'Dr. Paulo Medeiros', 'Nova frase de conclusão'), versao(1, TEXTO_ECO, 'substituida', 'Dr. Paulo Medeiros', null)],
    },
    {
      id: 'mod-tc', codigo: 'MOD-2026-000003', tipo: 'clinica', nome: 'TC de crânio sem contraste', exame: 'TC de crânio sem contraste', modalidade: 'CT', autor: 'Clínica',
      ativo: true, motivo_desativacao: null, versao_publicada: 5, proposta_em_aberto: 6, usos: 1102, pode_editar: false, pode_decidir: false, pode_desativar: false,
      versoes: [versao(6, `${TEXTO_TC}\nO método tem limitações.`, 'proposta', 'Dr. Paulo Medeiros', 'Inclui limitação do método'), versao(5, TEXTO_TC, 'publicada', 'Dra. Cecília Souto', 'Classificação atualizada', 'Dr. Marcos Freire')],
    },
  ];
}

function frasesIniciais(): FraseDeLaudo[] {
  return [
    { id: 'fra-normal', codigo: 'FRA-2026-000001', tipo: 'clinica', atalho: '.normal', texto: 'Exame dentro dos limites da normalidade para a idade e o sexo do paciente.', dono: 'Clínica', substitui_codigo: null, motivo: null, criado_em: '2026-09-01T12:00:00Z', pode_editar: false },
    { id: 'fra-retorno', codigo: 'FRA-2026-000002', tipo: 'pessoal', atalho: '.retorno', texto: 'Sugere-se controle ecográfico conforme critério do médico assistente.', dono: 'Dr. Paulo Medeiros', substitui_codigo: null, motivo: null, criado_em: '2026-09-02T12:00:00Z', pode_editar: true },
  ];
}

const ANTONIO = { id: 'pac-antonio', nome: 'Antônio Carlos Ribeiro', nascimento: '1954-06-15' };
const LAUDO_REVISADO =
  'Parênquima pulmonar com enfisema centrolobular leve nos lobos superiores. Nódulo sólido de 6 mm no segmento anterior do lobo superior direito. Impressão: enfisema leve; nódulo de 6 mm, controle conforme Fleischner.';

function filaInicial(): ItemDaFilaDeRevisao[] {
  return [
    { id: 'rev-1101', codigo: 'REV-2026-001101', laudo_codigo: 'LAU-2026-004120', exame: 'TC de tórax sem contraste', situacao: 'designada', classe: null, designada_em: '2026-11-02T12:00:00Z', concluida_em: null },
    { id: 'rev-1102', codigo: 'REV-2026-001102', laudo_codigo: 'LAU-2026-004188', exame: 'Raio X de tórax PA e perfil', situacao: 'designada', classe: null, designada_em: '2026-11-03T12:00:00Z', concluida_em: null },
    { id: 'rev-1095', codigo: 'REV-2026-001095', laudo_codigo: 'LAU-2026-003990', exame: 'TC de crânio sem contraste', situacao: 'concluida', classe: '1', designada_em: '2026-10-27T12:00:00Z', concluida_em: '2026-10-28T12:00:00Z' },
  ];
}

function meusLaudosIniciais(): RevisoesDoAutor {
  return {
    revisados: 6, concordantes: 5, concordancia_percentual: 83, pendentes: 1,
    revisoes: [
      { id: 'rev-1046', codigo: 'REV-2026-001046', laudo_id: 'laudo-1046', laudo_codigo: 'LAU-2026-003801', exame: 'Raio X de tórax', versao: 1, paciente: { id: 'pac-vera', nome: 'Vera Lúcia Antunes', nascimento: '1957-09-18' }, classe: '3a',
        comentario: 'Consolidação basal discreta à direita, não descrita.', concluida_em: '2026-10-30T12:00:00Z', pendencia: 'aguarda_autor', pendencia_motivo: null, pendencia_resolvida_em: null },
      { id: 'rev-1040', codigo: 'REV-2026-001040', laudo_id: 'laudo-1040', laudo_codigo: 'LAU-2026-003790', exame: 'Raio X de coluna lombar', versao: 1, paciente: { id: 'pac-rafael', nome: 'Rafael Monteiro Lima', nascimento: '1988-11-09' }, classe: '1',
        comentario: '', concluida_em: '2026-10-22T12:00:00Z', pendencia: null, pendencia_motivo: null, pendencia_resolvida_em: null },
    ],
  };
}

function painelInicial(): PainelDeRevisao {
  return {
    dias: 90,
    por_medico: [
      { medico_id: 'u-cecilia', medico: 'Dra. Cecília Souto', revisados: 9, concordantes: 9, concordancia_percentual: 100, discrepancias: 0, pendentes: 0 },
      { medico_id: 'u-felipe', medico: 'Dr. Felipe Andrade', revisados: 6, concordantes: 5, concordancia_percentual: 83, discrepancias: 1, pendentes: 1 },
    ],
    discrepancias: [
      { id: 'rev-1046', codigo: 'REV-2026-001046', laudo_codigo: 'LAU-2026-003801', exame: 'Raio X de tórax', classe: '3a', autor: 'Dr. Felipe Andrade', pendencia: 'aguarda_autor', concluida_em: '2026-10-30T12:00:00Z' },
      { id: 'rev-1052', codigo: 'REV-2026-001052', laudo_codigo: 'LAU-2026-003850', exame: 'USG de tireoide', classe: '2a', autor: 'Dra. Renata Alves', pendencia: null, concluida_em: '2026-10-29T12:00:00Z' },
    ],
    abertas: [
      { id: 'rev-1101', codigo: 'REV-2026-001101', laudo_id: 'laudo-4120', laudo_codigo: 'LAU-2026-004120', exame: 'TC de tórax sem contraste', revisor_id: 'u-marcos', revisor: 'Dr. Marcos Freire', modo: 'sorteio', situacao: 'designada', designada_em: '2026-11-02T12:00:00Z', motivo_conflito: null },
    ],
    conflitos: [],
    medicos: [{ id: 'u-cecilia', nome: 'Dra. Cecília Souto' }, { id: 'u-marcos', nome: 'Dr. Marcos Freire' }, { id: 'u-paulo', nome: 'Dr. Paulo Medeiros' }],
  };
}

export const modelosSimulados = { modelos: modelosIniciais(), frases: frasesIniciais(), fila: filaInicial(), meus: meusLaudosIniciais(), painel: painelInicial() };

export function reiniciarModelos(): void {
  Object.assign(modelosSimulados, { modelos: modelosIniciais(), frases: frasesIniciais(), fila: filaInicial(), meus: meusLaudosIniciais(), painel: painelInicial() });
}

function comPermissoes(m: ModeloDeLaudo): ModeloDeLaudo {
  const proposta = m.versoes.find((v) => v.situacao === 'proposta');
  return { ...m, pode_decidir: m.ativo && m.tipo === 'clinica' && ehCoordenacao() && proposta !== undefined && proposta.proposta_por !== 'Dra. Cecília Souto' };
}

function caso(id: string): CasoDeRevisao | null {
  const f = modelosSimulados.fila.find((i) => i.id === id);
  if (f === undefined) return null;
  const concluida = f.situacao === 'concluida';
  return {
    id: f.id, codigo: f.codigo, situacao: f.situacao, laudo_id: 'laudo-4120', laudo_codigo: f.laudo_codigo, exame: f.exame, versao: 1, paciente: ANTONIO,
    texto: f.situacao === 'designada' ? LAUDO_REVISADO : null, assinada_em: '2026-10-14T13:00:00Z', estudo_id: null,
    autor: concluida ? 'Dra. Cecília Souto' : null, classe: f.classe ?? null, comentario: null, concluida_em: f.concluida_em ?? null,
  };
}

export const manipuladoresModelos = [
  http.get('*/v1/modelos-de-laudo', () => (temPerfil('medico') ? HttpResponse.json(modelosSimulados.modelos.map(comPermissoes)) : semPermissao('ver os modelos de laudo'))),
  http.post('*/v1/modelos-de-laudo', async ({ request }) => {
    const n = (await registrar('criar-modelo', request)) as NovoModeloDeLaudo;
    if (!temPerfil('medico')) return semPermissao('criar modelo');
    if (/\d{3}\.\d{3}\.\d{3}-\d{2}|\d{2}\/\d{2}\/\d{4}/.test(n.texto)) {
      return problema(422, 'modelo-com-dado-de-paciente', 'O texto tem o que parece CPF ou data completa. Modelo e frase não levam dado de paciente: use um campo variável entre colchetes, como [data]. Nada foi gravado.');
    }
    const clinica = n.tipo === 'clinica';
    const m: ModeloDeLaudo = {
      id: `mod-${String(modelosSimulados.modelos.length + 1)}`, codigo: `MOD-2026-00000${String(modelosSimulados.modelos.length + 1)}`, tipo: n.tipo, nome: n.nome, exame: n.exame, modalidade: n.modalidade.toUpperCase(),
      autor: clinica ? 'Clínica' : estado().nome, ativo: true, motivo_desativacao: null, versao_publicada: clinica ? null : 1, proposta_em_aberto: clinica ? 1 : null, usos: 0,
      pode_editar: !clinica, pode_decidir: false, pode_desativar: !clinica, versoes: [versao(1, n.texto, clinica ? 'proposta' : 'publicada', estado().nome, null)],
    };
    modelosSimulados.modelos.push(m);
    return HttpResponse.json(comPermissoes(m), { status: 201 });
  }),
  http.post('*/v1/modelos-de-laudo/:id/versoes', async ({ request, params }) => {
    const p = (await registrar('nova-versao-de-modelo', request)) as NovaVersaoDeModelo;
    const m = modelosSimulados.modelos.find((x) => x.id === params.id);
    if (m === undefined) return problema(404, 'modelo-nao-encontrado', 'Este modelo não existe ou não está disponível para você. Nada foi alterado.');
    if (m.versao_publicada !== p.versao_base) return problema(409, 'versao-desatualizada', 'O modelo mudou enquanto você editava. Nada foi gravado. Abra a versão atual e refaça a mudança.');
    const numero = Math.max(...m.versoes.map((v) => v.numero)) + 1;
    const clinica = m.tipo === 'clinica';
    if (!clinica) m.versoes = m.versoes.map((v) => (v.situacao === 'publicada' ? { ...v, situacao: 'substituida' as const } : v));
    m.versoes = [versao(numero, p.texto, clinica ? 'proposta' : 'publicada', estado().nome, p.motivo), ...m.versoes];
    if (clinica) {
      m.proposta_em_aberto = numero;
      m.pode_editar = false;
    } else m.versao_publicada = numero;
    return HttpResponse.json(comPermissoes(m), { status: 201 });
  }),
  http.post('*/v1/modelos-de-laudo/:id/versoes/:numero/decisao', async ({ request, params }) => {
    const p = (await registrar('decidir-versao-de-modelo', request)) as { aprovar: boolean; motivo?: string | null };
    if (!ehCoordenacao()) return problema(403, 'coordenacao-medica', 'Este ato é da coordenação médica (médico com perfil de coordenação). Nada foi alterado.');
    const m = modelosSimulados.modelos.find((x) => x.id === params.id);
    if (m === undefined) return problema(404, 'modelo-nao-encontrado', 'Modelo não encontrado.');
    const numero = Number(params.numero);
    m.versoes = m.versoes.map((v) => {
      if (v.numero === numero) return { ...v, situacao: p.aprovar ? ('publicada' as const) : ('recusada' as const), decidido_por: estado().nome, decidido_em: '2026-11-07T13:00:00Z', motivo_decisao: p.motivo ?? null };
      if (p.aprovar && v.situacao === 'publicada') return { ...v, situacao: 'substituida' as const };
      return v;
    });
    m.proposta_em_aberto = null;
    m.pode_editar = true;
    if (p.aprovar) m.versao_publicada = numero;
    return HttpResponse.json(comPermissoes(m));
  }),
  http.post('*/v1/modelos-de-laudo/:id/desativacao', async ({ request, params }) => {
    const p = (await registrar('desativar-modelo', request)) as { motivo: string };
    const m = modelosSimulados.modelos.find((x) => x.id === params.id);
    if (m === undefined) return problema(404, 'modelo-nao-encontrado', 'Modelo não encontrado.');
    Object.assign(m, { ativo: false, motivo_desativacao: p.motivo, pode_editar: false, pode_desativar: false });
    return HttpResponse.json(comPermissoes(m));
  }),
  http.post('*/v1/laudos/:id/usos-de-modelo', async ({ request }) => {
    const p = (await registrar('usar-modelo', request)) as { modelo_id: string; numero: number };
    const m = modelosSimulados.modelos.find((x) => x.id === p.modelo_id);
    const v = m?.versoes.find((x) => x.numero === p.numero && x.situacao === 'publicada');
    if (m === undefined || v === undefined) return problema(409, 'versao-de-modelo-indisponivel', 'Esta versão do modelo não está mais publicada. Nada foi inserido. Recarregue os modelos e escolha de novo.');
    return HttpResponse.json({ modelo_codigo: m.codigo, numero: v.numero, laudo_versao: 1, texto: v.texto }, { status: 201 });
  }),
  http.get('*/v1/frases-de-laudo', () => (temPerfil('medico') ? HttpResponse.json(modelosSimulados.frases) : semPermissao('ver as frases'))),
  http.post('*/v1/frases-de-laudo', async ({ request }) => {
    const n = (await registrar('criar-frase', request)) as { tipo: 'clinica' | 'pessoal'; atalho: string; texto: string };
    if (modelosSimulados.frases.some((f) => f.atalho === n.atalho)) return problema(409, 'atalho-em-uso', 'Este atalho já está em uso. Nada foi criado. Escolha outro ou edite a frase existente.');
    const f: FraseDeLaudo = { id: `fra-${n.atalho}`, codigo: 'FRA-2026-000009', tipo: n.tipo, atalho: n.atalho, texto: n.texto, dono: estado().nome, substitui_codigo: null, motivo: null, criado_em: '2026-11-07T12:00:00Z', pode_editar: true };
    modelosSimulados.frases.push(f);
    return HttpResponse.json(f, { status: 201 });
  }),
  http.get('*/v1/revisoes-por-pares/fila', () => (temPerfil('medico') ? HttpResponse.json(modelosSimulados.fila) : semPermissao('ver a fila de revisão'))),
  http.get('*/v1/revisoes-por-pares/meus-laudos', () => (temPerfil('medico') ? HttpResponse.json(modelosSimulados.meus) : semPermissao('ver as revisões'))),
  http.get('*/v1/revisoes-por-pares/painel', () =>
    ehCoordenacao() ? HttpResponse.json(modelosSimulados.painel) : problema(403, 'coordenacao-medica', 'Este ato é da coordenação médica (médico com perfil de coordenação). Nada foi alterado.'),
  ),
  http.post('*/v1/revisoes-por-pares/sorteio', async ({ request }) => {
    await registrar('sortear-revisoes', request);
    const designadas = modelosSimulados.painel.abertas;
    return HttpResponse.json({ candidatos: 4, designadas, sem_revisor: 0 }, { status: 201 });
  }),
  http.post('*/v1/revisoes-por-pares', async ({ request }) => {
    await registrar('designar-revisao', request);
    return problema(422, 'revisor-e-autor', 'Quem laudou não revisa o próprio laudo. Nada foi designado. Escolha outro médico.');
  }),
  http.get('*/v1/revisoes-por-pares/:id', ({ params }) => {
    if (!temPerfil('medico')) return semPermissao('abrir a revisão');
    const c = caso(String(params.id));
    return c === null ? problema(404, 'revisao-nao-encontrada', 'Esta revisão não existe nesta unidade ou não é sua. Nada foi aberto.') : HttpResponse.json(c);
  }),
  http.post('*/v1/revisoes-por-pares/:id/parecer', async ({ request, params }) => {
    const p = (await registrar('parecer-de-revisao', request)) as ParecerDeRevisao;
    if (p.classe !== '1' && p.comentario.trim().length < 10) {
      return problema(422, 'comentario-obrigatorio', 'Na discrepância, explique ao colega o que foi visto (10 letras ou mais). Nada foi enviado.');
    }
    modelosSimulados.fila = modelosSimulados.fila.map((f) => (f.id === params.id ? { ...f, situacao: 'concluida' as const, classe: p.classe, concluida_em: '2026-11-07T13:00:00Z' } : f));
    const c = caso(String(params.id));
    return c === null ? problema(404, 'revisao-nao-encontrada', 'Revisão não encontrada.') : HttpResponse.json({ ...c, comentario: p.comentario });
  }),
  http.post('*/v1/revisoes-por-pares/:id/conflito', async ({ request, params }) => {
    await registrar('conflito-de-revisao', request);
    modelosSimulados.fila = modelosSimulados.fila.map((f) => (f.id === params.id ? { ...f, situacao: 'conflito_declarado' as const } : f));
    return HttpResponse.json(modelosSimulados.fila.find((f) => f.id === params.id));
  }),
  http.post('*/v1/revisoes-por-pares/:id/pendencia', async ({ request, params }) => {
    const p = (await registrar('pendencia-de-revisao', request)) as { decisao: string; motivo?: string | null };
    if (p.decisao === 'retificado') {
      return problema(409, 'retificacao-nao-encontrada', 'O laudo ainda não tem versão nova. Nada foi alterado. Abra a retificação no laudo, com motivo, e depois marque como retificado.');
    }
    const r = modelosSimulados.meus.revisoes.find((x) => x.id === params.id);
    if (r === undefined) return problema(404, 'revisao-nao-encontrada', 'Revisão não encontrada.');
    Object.assign(r, { pendencia: 'mantido', pendencia_motivo: p.motivo ?? null, pendencia_resolvida_em: '2026-11-07T14:00:00Z' });
    modelosSimulados.meus.pendentes = 0;
    return HttpResponse.json(r);
  }),
];

function estado(): { nome: string } {
  return { nome: nucleo.sessao.usuario.nome };
}
