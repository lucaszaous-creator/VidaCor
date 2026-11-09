/**
 * Núcleo simulado de Convênios e tabelas e de Check-up e pacotes. Só imita o contrato para conferir o
 * que a tela mostra; as regras (reajuste, segunda pessoa, avulso, limites, concorrência) são provadas em
 * `crates/api/tests/convenios.rs` e `pacotes.rs`, com banco real (G4). Só dados fictícios.
 */
import type {
  AlteracaoDoConvenio,
  CatalogoDePacotes,
  ConvenioDetalhe,
  ConvenioResumo,
  DecisaoDaPropostaDeTabela,
  NovaPropostaDeTabela,
  NovaVersaoDoPacoteDeCheckup,
  NovoConvenio,
  NovoPacoteDeCheckup,
} from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { convenioUnimed, conveniosIniciais, pacotesIniciais, precosUnimed, propostaUnimed } from './dados/convenios';
import { reautenticacao, registrar, semPermissao, temPerfil } from './nucleo-fase1';
import { estado } from './nucleo-simulado';
import { problema } from './problema';

export const conveniosSimulados: { lista: ConvenioResumo[]; unimed: ConvenioDetalhe; pacotes: CatalogoDePacotes } = {
  lista: conveniosIniciais(),
  unimed: convenioUnimed(),
  pacotes: pacotesIniciais(),
};

/** Começa com a proposta de reajuste da Unimed aguardando a gerência. */
export function comPropostaPendente(): void {
  conveniosSimulados.lista = conveniosIniciais(true);
  conveniosSimulados.unimed = convenioUnimed(true);
}

export function reiniciarConvenios(): void {
  conveniosSimulados.lista = conveniosIniciais();
  conveniosSimulados.unimed = convenioUnimed();
  conveniosSimulados.pacotes = pacotesIniciais();
}

const leConvenios = () => temPerfil('faturista', 'gerente');
const naoAchou = () => problema(404, 'convenio-nao-encontrado', 'Convênio não encontrado nesta unidade. Nada foi alterado.');

export const manipuladoresConvenios = [
  http.get('*/v1/convenios', () => (leConvenios() ? HttpResponse.json(conveniosSimulados.lista) : semPermissao('consultar os convênios'))),
  http.post('*/v1/convenios/propostas/:id/decisao', async ({ request, params }) => {
    const c = (await registrar('decidir-tabela', request)) as DecisaoDaPropostaDeTabela;
    if (!temPerfil('gerente')) return semPermissao('decidir a proposta de tabela');
    const p = conveniosSimulados.unimed.proposta_pendente;
    if (p === null || p === undefined || p.id !== params.id) return problema(404, 'proposta-nao-encontrada', 'Proposta não encontrada nesta unidade. Nada foi alterado.');
    if (p.proposta_por_id === estado.sessao.usuario.id) {
      return problema(403, 'separacao-de-funcoes', 'A aprovação da tabela é de outra pessoa: quem propôs não decide. Nada foi alterado. Peça a outra pessoa da gerência.');
    }
    const r = reautenticacao('decidir-tabela');
    if (r !== null) return r;
    const decidida = { ...p, situacao: c.aprovar ? ('aprovada' as const) : ('recusada' as const), decidido_por: 'Patrícia Lemos', decidido_em: '2026-11-07T12:00:00Z', motivo_decisao: c.motivo ?? null, tabela_versao: c.aprovar ? 15 : null };
    conveniosSimulados.unimed = { ...conveniosSimulados.unimed, proposta_pendente: null };
    return HttpResponse.json(decidida);
  }),
  http.get('*/v1/convenios/:id', ({ params }) => {
    if (!leConvenios()) return semPermissao('consultar os convênios');
    if (params.id === conveniosSimulados.unimed.id) return HttpResponse.json(conveniosSimulados.unimed);
    const k = conveniosSimulados.lista.find((x) => x.id === params.id);
    if (k === undefined) return naoAchou();
    return HttpResponse.json({ ...convenioUnimed(), id: k.id, codigo: k.codigo, operadora: k.operadora, registro_ans: k.registro_ans, situacao: k.situacao, proposta_pendente: null });
  }),
  http.get('*/v1/convenios/:id/precos', () => (leConvenios() ? HttpResponse.json(precosUnimed()) : semPermissao('consultar os convênios'))),
  http.get('*/v1/convenios/:id/previa-de-reajuste', ({ request }) => {
    if (!leConvenios()) return semPermissao('consultar os convênios');
    const pb = Number(new URL(request.url).searchParams.get('pontos_base'));
    if (!(pb >= 1 && pb <= 3000)) return problema(422, 'reajuste-invalido', 'O reajuste em lote vai de 0,01% a 30%. Nada foi calculado.');
    const p = propostaUnimed();
    return HttpResponse.json({ base_versao: 14, pontos_base: pb, afetados: 142, total_antes_centavos: p.total_antes_centavos, total_depois_centavos: p.total_depois_centavos, maiores: p.maiores });
  }),
  http.post('*/v1/convenios/:id/propostas', async ({ request }) => {
    const c = (await registrar('propor-tabela', request)) as NovaPropostaDeTabela;
    if (!temPerfil('faturista')) return semPermissao('propor tabela');
    if (conveniosSimulados.unimed.proposta_pendente !== null && conveniosSimulados.unimed.proposta_pendente !== undefined) {
      return problema(409, 'proposta-pendente', 'Este convênio já tem proposta de tabela aguardando aprovação. Nada foi proposto.');
    }
    const p = { ...propostaUnimed(), vigente_desde: c.vigente_desde, descricao: c.descricao, documento: c.documento, motivo: c.motivo, reajuste_pontos_base: c.reajuste_pontos_base ?? null };
    conveniosSimulados.unimed = { ...conveniosSimulados.unimed, proposta_pendente: p };
    return HttpResponse.json(p, { status: 201 });
  }),
  http.post('*/v1/convenios/:id/alteracoes', async ({ request }) => {
    const c = (await registrar('alterar-convenio', request)) as AlteracaoDoConvenio;
    if (!temPerfil('faturista')) return semPermissao('mudar o convênio');
    const u = conveniosSimulados.unimed;
    if (c.versao !== u.versao) return problema(409, 'versao-mudou', 'Outra pessoa mudou este convênio enquanto você editava. Nada foi alterado. Recarregue e confira antes de repetir.');
    conveniosSimulados.unimed = { ...u, versao: u.versao + 1, termos: c.termos };
    return HttpResponse.json(conveniosSimulados.unimed);
  }),
  http.post('*/v1/convenios/:id/regras-de-autorizacao', async ({ request }) => {
    await registrar('publicar-regras', request);
    if (!temPerfil('faturista')) return semPermissao('publicar regras');
    const r = conveniosSimulados.unimed.regras_autorizacao;
    if (r === null || r === undefined) return naoAchou();
    const nova = { ...r, versao_regras: r.versao_regras + 1 };
    conveniosSimulados.unimed = { ...conveniosSimulados.unimed, regras_autorizacao: nova };
    return HttpResponse.json(nova, { status: 201 });
  }),
  http.post('*/v1/convenios', async ({ request }) => {
    const c = (await registrar('cadastrar-convenio', request)) as NovoConvenio;
    if (!temPerfil('faturista')) return semPermissao('cadastrar operadora');
    if (conveniosSimulados.lista.some((k) => k.operadora.toLocaleLowerCase('pt-BR') === c.operadora.trim().toLocaleLowerCase('pt-BR'))) {
      return problema(409, 'operadora-ja-cadastrada', 'Esta operadora já tem convênio nesta unidade. Nada foi gravado. Abra o convênio na lista e, se preciso, reative.');
    }
    const novo = { ...convenioUnimed(), id: 'cvn-novo', codigo: 'CVN-2026-000009', operadora: c.operadora.trim(), registro_ans: c.registro_ans, termos: c.termos, tabelas: [], historico: [], regras_autorizacao: null, versao: 1 };
    conveniosSimulados.lista = [...conveniosSimulados.lista, { id: novo.id, codigo: novo.codigo, operadora: novo.operadora, registro_ans: novo.registro_ans, situacao: 'vigente', vigencia_fim: null, tabela_versao: null, proposta_pendente: false }];
    return HttpResponse.json(novo, { status: 201 });
  }),

  http.get('*/v1/pacotes', () => (temPerfil('recepcao', 'gerente') ? HttpResponse.json(conveniosSimulados.pacotes) : semPermissao('consultar os pacotes'))),
  http.post('*/v1/pacotes/:codigo/versoes', async ({ request, params }) => {
    const c = (await registrar('nova-versao-pacote', request)) as NovaVersaoDoPacoteDeCheckup;
    if (!temPerfil('gerente')) return semPermissao('publicar pacote');
    const p = conveniosSimulados.pacotes.pacotes.find((x) => x.codigo === params.codigo);
    const topo = p?.versoes[0];
    if (p === undefined || topo === undefined) return problema(404, 'pacote-nao-encontrado', 'Pacote não encontrado nesta unidade. Nada foi publicado.');
    if (c.regra_versao_vista !== topo.regra_versao) return problema(409, 'versao-mudou', 'Outra pessoa publicou versão deste pacote enquanto você editava. Nada foi publicado.');
    if (c.preco_centavos >= 89_000) return problema(422, 'pacote-nao-sai-mais-barato', 'O preço do pacote precisa ser menor que a soma avulsa dos exames; senão o orçamento não o aplica. Nada foi publicado.');
    p.versoes = [{ ...topo, id: 'nova', regra_versao: topo.regra_versao + 1, situacao: 'futura', vigente_desde: c.vigente_desde, preco_centavos: c.preco_centavos, motivo: c.motivo }, ...p.versoes];
    return HttpResponse.json(p, { status: 201 });
  }),
  http.post('*/v1/pacotes', async ({ request }) => {
    const c = (await registrar('criar-pacote', request)) as NovoPacoteDeCheckup;
    if (!temPerfil('gerente')) return semPermissao('criar pacote');
    const base = pacotesIniciais().pacotes[0]?.versoes[1];
    if (base === undefined) return problema(500, 'falha', 'falha');
    const novo = { codigo: 'PCT-2026-000009', nome: c.nome, versoes: [{ ...base, id: 'novo', regra_versao: 1, nome: c.nome, para_quem: c.para_quem, vigente_desde: c.vigente_desde, preco_centavos: c.preco_centavos, motivo: c.motivo }] };
    conveniosSimulados.pacotes.pacotes = [...conveniosSimulados.pacotes.pacotes, novo];
    return HttpResponse.json(novo, { status: 201 });
  }),
];
