/**
 * Núcleo simulado do laudo estruturado (achados TI-RADS e BI-RADS). Imita só o que a tela precisa
 * mostrar: a categoria vem na resposta, laudo assinado recusa, só o médico atribuído escreve. As
 * regras de verdade (tabela, coerência, versão da regra, RLS) são provadas no domínio
 * (`crates/dominio-laudo/tests`) e nos testes G4 em `crates/api/tests/laudo_estruturado.rs`.
 */
import type { AchadoEstruturado, AchadosDoLaudo, NovoAchado } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { uuid } from './dados/ficticios';
import { fase1, registrar, temPerfil } from './nucleo-fase1';
import { estado } from './nucleo-simulado';
import { problema } from './problema';

export const laudoEstruturado = { achados: [] as (AchadoEstruturado & { removido: boolean })[], porChave: new Map<string, AchadoEstruturado>() };

export function reiniciarLaudoEstruturado(): void {
  laudoEstruturado.achados = [];
  laudoEstruturado.porChave.clear();
}

const REGRAS = [
  { sistema: 'tirads' as const, versao: 'TIRADS-ACR-2017.1' },
  { sistema: 'birads' as const, versao: 'BIRADS-ACR-2013-US.1' },
];

/** Resposta pronta por combinação usada nos testes de tela (a tabela completa é do servidor). */
function resultadoSimulado(c: NovoAchado): Pick<AchadoEstruturado, 'pontos' | 'categoria' | 'categoria_nome' | 'conduta' | 'regra_versao'> | null {
  if (c.sistema === 'tirads' && c.tirads) {
    const solidoHipo = c.tirads.composicao === 'solida_ou_quase_solida' && c.tirads.ecogenicidade === 'hipoecoica';
    return solidoHipo
      ? { pontos: 4, categoria: 'TR4', categoria_nome: 'moderadamente suspeito', conduta: 'PAAF indicada (a partir de 15 mm)', regra_versao: 'TIRADS-ACR-2017.1' }
      : { pontos: 0, categoria: 'TR1', categoria_nome: 'benigno', conduta: 'sem indicação de PAAF ou de seguimento', regra_versao: 'TIRADS-ACR-2017.1' };
  }
  if (c.sistema === 'birads' && c.birads) {
    if (c.birads.categoria_atribuida === '6' && !c.birads.biopsia_maligna_confirmada) return null;
    return {
      pontos: null,
      categoria: c.birads.categoria_atribuida,
      categoria_nome: 'alta suspeita de malignidade',
      conduta: 'diagnóstico tecidual (biópsia); probabilidade de malignidade: acima de 50% e abaixo de 95%',
      regra_versao: 'BIRADS-ACR-2013-US.1',
    };
  }
  return null;
}

function lista(): AchadosDoLaudo {
  const ativos: AchadoEstruturado[] = laudoEstruturado.achados
    .filter((a) => !a.removido)
    .map((a) => {
      const copia: Partial<typeof a> = { ...a };
      delete copia.removido;
      return copia as AchadoEstruturado;
    });
  return {
    laudo_id: fase1.laudo.id,
    versao_numero: fase1.laudo.versoes.length,
    editavel: fase1.laudo.situacao !== 'assinado' && estado.sessao.usuario.nome === 'Dr. Paulo Medeiros',
    achados: ativos,
    texto_achados: ativos.map((a) => a.texto_achado).join('\n'),
    texto_conclusao: ativos.map((a) => a.texto_conclusao).join('\n'),
    regras_vigentes: REGRAS,
  };
}

export const manipuladoresLaudoEstruturado = [
  http.get('*/v1/laudos/:id/achados', () => {
    if (!temPerfil('medico')) return problema(403, 'sem-permissao', 'Seu perfil não lê laudos. Nada foi aberto.');
    return HttpResponse.json(lista());
  }),
  http.post('*/v1/laudos/:id/achados', async ({ request }) => {
    const corpo = (await registrar('achado', request)) as NovoAchado;
    const chave = request.headers.get('Idempotency-Key') ?? '';
    const repetido = laudoEstruturado.porChave.get(chave);
    if (repetido !== undefined) return HttpResponse.json(repetido, { status: 201 });
    if (estado.sessao.usuario.nome !== 'Dr. Paulo Medeiros') {
      return problema(403, 'laudo-de-outro-medico', 'Este laudo está atribuído a outro médico. Nada foi alterado.');
    }
    if (fase1.laudo.situacao === 'assinado') {
      return problema(409, 'laudo-assinado', 'Este laudo está assinado e não recebe nem perde achados. Nada foi alterado. Para corrigir, faça uma retificação com motivo.');
    }
    const r = resultadoSimulado(corpo);
    if (r === null) {
      return problema(422, 'birads-6-sem-biopsia', 'BI-RADS 6 exige malignidade confirmada por biópsia. Confirme a biópsia ou escolha outra categoria. Nada foi gravado.');
    }
    const numero = laudoEstruturado.achados.length + 1;
    const rotulo = `N${String(numero)}`;
    const achado: AchadoEstruturado = {
      id: uuid('000000e2', numero),
      codigo: `ACH-2026-${String(numero).padStart(6, '0')}`,
      numero,
      rotulo,
      versao_numero: fase1.laudo.versoes.length,
      sistema: corpo.sistema,
      localizacao: corpo.localizacao,
      maior_diametro_mm: corpo.maior_diametro_mm ?? null,
      tirads: corpo.tirads ?? null,
      birads: corpo.birads ?? null,
      ...r,
      texto_achado: `${rotulo}: ${corpo.localizacao.toLocaleLowerCase('pt-BR')}, categoria ${r.categoria}.`,
      texto_conclusao: `${rotulo}: ${r.categoria}, ${r.categoria_nome}; ${r.conduta}.`,
      criado_em: '2026-11-06T12:00:00Z',
    };
    laudoEstruturado.achados.push({ ...achado, removido: false });
    laudoEstruturado.porChave.set(chave, achado);
    return HttpResponse.json(achado, { status: 201 });
  }),
  http.post('*/v1/laudos/:id/achados/:achadoId/remocao', async ({ request, params }) => {
    await registrar('remocao-achado', request);
    const a = laudoEstruturado.achados.find((x) => x.id === params.achadoId);
    if (a === undefined) return problema(404, 'achado-nao-encontrado', 'Este achado não existe neste laudo. Nada foi alterado.');
    a.removido = true;
    return HttpResponse.json(lista());
  }),
];
