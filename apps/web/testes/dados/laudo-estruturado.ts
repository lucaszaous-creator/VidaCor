/**
 * Achado estruturado fictício (N1 do mockup Laudo-Estruturado.dc.html) para a verificação de que a
 * tela cabe na janela. Os valores são os que o servidor devolveria; aqui não se calcula nada.
 */
import type { AchadoEstruturado, AchadosDoLaudo } from '@vidacor/api-client';
import { uuid } from './ficticios.ts';

export function achadoN1(): AchadoEstruturado {
  return {
    id: uuid('000000e2', 1),
    codigo: 'ACH-2026-000001',
    numero: 1,
    rotulo: 'N1',
    versao_numero: 1,
    sistema: 'tirads',
    localizacao: 'Lobo direito, terço médio',
    maior_diametro_mm: 18,
    tirads: { composicao: 'solida_ou_quase_solida', ecogenicidade: 'hipoecoica', forma: 'mais_largo_que_alto', margem: 'lisa', focos: ['nenhum_ou_cauda_de_cometa'] },
    birads: null,
    pontos: 4,
    categoria: 'TR4',
    categoria_nome: 'moderadamente suspeito',
    conduta: 'PAAF indicada (a partir de 15 mm)',
    regra_versao: 'TIRADS-ACR-2017.1',
    texto_achado:
      'N1: no lobo direito, terço médio, nódulo de 18 mm no maior eixo, sólido ou quase todo sólido, hipoecoico, mais largo que alto, margens lisas, sem focos ecogênicos. ACR TI-RADS TR4 (4 pontos).',
    texto_conclusao: 'N1: ACR TI-RADS TR4, moderadamente suspeito; PAAF indicada (a partir de 15 mm).',
    criado_em: '2026-11-06T12:00:00Z',
  };
}

export function achadosDoLaudoAntonio(laudoId: string): AchadosDoLaudo {
  const a = achadoN1();
  return {
    laudo_id: laudoId,
    versao_numero: 1,
    editavel: true,
    achados: [a],
    texto_achados: a.texto_achado,
    texto_conclusao: a.texto_conclusao,
    regras_vigentes: [
      { sistema: 'tirads', versao: 'TIRADS-ACR-2017.1' },
      { sistema: 'birads', versao: 'BIRADS-ACR-2013-US.1' },
    ],
  };
}
