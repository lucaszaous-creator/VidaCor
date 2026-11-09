/**
 * Comunicação com pacientes simulada para os testes de TELA. As regras reais (aviso desligado não sai,
 * versão vista, acesso indevido) estão em `crates/api/tests/comunicacao_painel.rs`.
 */
import type { AvisoAutomatico, MudancaDeAviso, PainelDeComunicacao } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { semPermissao, temPerfil } from './nucleo-fase1';
import { problema } from './problema';

function avisosIniciais(): AvisoAutomatico[] {
  return [
    { modelo: 'confirmacao', nome: 'Confirmação do horário', descricao: 'Sai quando a recepção pede a confirmação dos horários do dia.', ativo: true, versao_id: null },
    { modelo: 'laudo_pronto', nome: 'Laudo pronto no portal', descricao: 'Sai quando o laudo assinado é publicado no portal.', ativo: false, motivo: 'Portal em manutenção programada.', alterado_por: 'Patrícia Lemos', alterado_em: '2026-11-08T12:00:00Z', versao_id: 'v1' },
  ];
}

export const comunicacaoDoPainel: { avisos: AvisoAutomatico[]; mudancas: MudancaDeAviso[] } = { avisos: avisosIniciais(), mudancas: [] };

export function reiniciarComunicacao(): void {
  comunicacaoDoPainel.avisos = avisosIniciais();
  comunicacaoDoPainel.mudancas = [];
}

export const manipuladoresComunicacaoPainel = [
  http.get('*/v1/comunicacao', ({ request }) => {
    if (!temPerfil('recepcao', 'gerente')) return semPermissao('ver a comunicação com pacientes');
    const filtro = new URL(request.url).searchParams.get('filtro') ?? 'todas';
    const todas: PainelDeComunicacao['mensagens'] = [
      { id: 'm1', paciente_id: 'p1', nome: 'Antônio Carlos Ribeiro', nascimento: '1954-06-15', modelo: 'confirmacao', situacao: 'enviada', resposta: 'confirmou', criado_em: '2026-11-09T12:00:00Z' },
      { id: 'm2', paciente_id: 'p2', nome: 'Maria Helena Duarte', nascimento: '1968-03-12', modelo: 'confirmacao', situacao: 'sem_celular', resposta: null, criado_em: '2026-11-09T12:00:00Z' },
    ];
    const painel: PainelDeComunicacao = {
      whatsapp_configurado: true,
      hoje: { enviadas: 1, confirmaram: 1, opt_out: 0, sem_celular: 1, desligadas: 0 },
      mensagens: filtro === 'nao_entregues' ? todas.filter((m) => m.situacao !== 'enviada') : filtro === 'confirmaram' ? todas.filter((m) => m.resposta === 'confirmou') : todas,
      automaticas: comunicacaoDoPainel.avisos,
      pacientes_sem_whatsapp: 4,
    };
    return HttpResponse.json(painel);
  }),
  http.post('*/v1/comunicacao/automaticas/:modelo', async ({ params, request }) => {
    if (!temPerfil('gerente')) return semPermissao('ligar ou desligar avisos');
    const m = (await request.json()) as MudancaDeAviso;
    comunicacaoDoPainel.mudancas.push(m);
    const a = comunicacaoDoPainel.avisos.find((x) => x.modelo === params.modelo);
    if (a === undefined) return problema(404, 'aviso-desconhecido', 'Este aviso automático não existe.');
    if ((a.versao_id ?? null) !== m.versao_vista) return problema(409, 'aviso-mudou', 'Outra pessoa mudou este aviso enquanto você via a tela. Nada foi gravado; veja como está agora e decida de novo.');
    const novo: AvisoAutomatico = { ...a, ativo: m.ativo, motivo: m.motivo, alterado_por: 'Patrícia Lemos', alterado_em: '2026-11-09T13:00:00Z', versao_id: `v${String(comunicacaoDoPainel.mudancas.length + 1)}` };
    comunicacaoDoPainel.avisos = comunicacaoDoPainel.avisos.map((x) => (x.modelo === a.modelo ? novo : x));
    return HttpResponse.json(novo);
  }),
];
