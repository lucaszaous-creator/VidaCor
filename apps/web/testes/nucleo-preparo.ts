/**
 * Núcleo simulado do preparo de exames e dos documentos do atendimento (comprovante, declaração e
 * recibo) para os testes de TELA. Regras reais em `crates/api/tests/documentos_atendimento.rs`, com
 * PostgreSQL real. Só dados fictícios.
 */
import type { NovoPreparo, PreparoDoExame, VersaoDoPreparo } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { registrar, semPermissao, temPerfil } from './nucleo-fase1';
import { estado } from './nucleo-simulado';
import { problema } from './problema';
import { SHA_FICTICIO } from './nucleo-documentos';

function versao(numero: number, passos: string[], vigente: boolean, motivo: string | null, por = 'Patrícia Lemos'): VersaoDoPreparo {
  return {
    id: `0000000c-0000-7000-8000-00000000000${String(numero)}`,
    codigo: `PRP-2026-00000${String(numero)}`,
    numero,
    passos,
    chegar_antes_min: numero === 1 ? 15 : 30,
    vigente,
    motivo,
    criado_em: `2026-11-0${String(numero)}T13:00:00Z`,
    criado_por: por,
  };
}

function iniciais(): { exames: PreparoDoExame[]; versoes: Record<string, VersaoDoPreparo[]> } {
  const v2 = versao(
    2,
    [
      'Venha com roupa leve e tênis confortável para caminhar na esteira.',
      'Faça uma refeição leve até 2 horas antes. Não precisa de jejum.',
      'Não fume e evite café nas 3 horas anteriores.',
      'Traga o pedido médico, um documento com foto e a carteirinha.',
    ],
    true,
    'Orientação revisada pelo responsável técnico',
  );
  const v1 = versao(1, ['Venha com roupa leve.', 'Traga o pedido médico.'], false, null);
  return {
    exames: [
      { tuss: '40901220', nome: 'Ecocardiograma com estresse', modalidade: 'US', vigente: v2 },
      { tuss: '40901114', nome: 'Ecocardiograma transtorácico', modalidade: 'US', vigente: null },
      { tuss: '41001109', nome: 'TC de abdome com contraste', modalidade: 'CT', vigente: null },
    ],
    versoes: { '40901220': [v2, v1] },
  };
}

export const preparoSimulado = iniciais();

export function reiniciarPreparo(): void {
  Object.assign(preparoSimulado, iniciais());
}

function pdf(nome: string, codigo: string) {
  return new HttpResponse(new Uint8Array([0x25, 0x50, 0x44, 0x46, 0x2d, 0x31, 0x2e, 0x37]), {
    status: 200,
    headers: {
      'Content-Type': 'application/pdf',
      'Content-Disposition': `attachment; filename="${nome}"`,
      'X-Documento-Sha256': SHA_FICTICIO,
      'X-Documento-Codigo': codigo,
    },
  });
}

export const manipuladoresPreparo = [
  http.get('*/v1/preparos', () => (temPerfil('recepcao', 'gerente') ? HttpResponse.json(preparoSimulado.exames) : semPermissao('consultar o preparo'))),
  http.get('*/v1/preparos/:tuss', ({ params }) => {
    if (!temPerfil('recepcao', 'gerente')) return semPermissao('consultar o preparo');
    const tuss = String(params.tuss);
    if (!preparoSimulado.exames.some((e) => e.tuss === tuss)) return problema(404, 'exame-nao-encontrado', 'Este exame não está no catálogo.');
    return HttpResponse.json(preparoSimulado.versoes[tuss] ?? []);
  }),
  http.post('*/v1/preparos/:tuss', async ({ request, params }) => {
    const n = (await registrar('preparo', request)) as NovoPreparo;
    if (!temPerfil('gerente')) return semPermissao('publicar o preparo');
    const exame = preparoSimulado.exames.find((e) => e.tuss === String(params.tuss));
    if (exame === undefined) return problema(404, 'exame-nao-encontrado', 'Este exame não está no catálogo.');
    const atual = exame.vigente ?? null;
    if ((n.versao_vista ?? null) !== (atual?.numero ?? null)) {
      return problema(409, 'preparo-mudou', 'Outra pessoa publicou uma versão do preparo enquanto você editava. Nada foi publicado. Recarregue, confira a versão nova e repita se ainda for preciso.');
    }
    if (atual !== null && (n.motivo ?? '').trim().length < 5) {
      return problema(422, 'motivo-obrigatorio', 'Diga por que o preparo mudou (5 a 300 caracteres). Nada foi publicado.');
    }
    const nova = { ...versao((atual?.numero ?? 0) + 1, n.passos, true, n.motivo ?? null, estado.sessao.usuario.nome), chegar_antes_min: n.chegar_antes_min };
    const anteriores = (preparoSimulado.versoes[exame.tuss] ?? []).map((v) => ({ ...v, vigente: false }));
    preparoSimulado.versoes[exame.tuss] = [nova, ...anteriores];
    exame.vigente = nova;
    return HttpResponse.json(nova, { status: 201 });
  }),
  http.get('*/v1/agendamentos/:id/comprovante', () =>
    temPerfil('recepcao', 'gerente') ? pdf('AGD-2026-000123-CPV-2026-000004.pdf', 'CPV-2026-000004') : semPermissao('emitir o comprovante'),
  ),
  http.get('*/v1/atendimentos/:id/declaracao', () =>
    temPerfil('recepcao', 'gerente') ? pdf('ATD-2026-000001-DCL-2026-000002.pdf', 'DCL-2026-000002') : semPermissao('emitir a declaração'),
  ),
  http.get('*/v1/caixa/recebimentos/:id/recibo', () =>
    temPerfil('caixa') ? pdf('REC-2026-000001-RCB-2026-000001.pdf', 'RCB-2026-000001') : semPermissao('emitir o recibo'),
  ),
];
