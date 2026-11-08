import type { ItemDoMenu, SecaoDoMenu } from '@vidacor/design-system';

/**
 * Itens do menu lateral (Sidebar.dc.html: mesmos nomes, ícones e cores por módulo do docs/05).
 * Mostrar só o que o perfil usa é CONFORTO: quem decide se o ato é permitido é o servidor, em cada
 * rota (G1). Por isso um perfil desconhecido não ganha item nenhum, e cada tela trata o 403.
 * Perfis conforme docs/18 § 3 e § 5.
 */
interface ItemComPerfis extends ItemDoMenu {
  perfis: readonly string[];
}

const RECEPCAO = ['recepcao', 'supervisor_recepcao', 'coordenacao'] as const;

const SECOES: { id: string; titulo: string; itens: ItemComPerfis[] }[] = [
  {
    id: 'clinica',
    titulo: 'Clínica',
    itens: [
      {
        id: 'agenda',
        rotulo: 'Agenda e check-in',
        href: '/agenda',
        icone: 'agenda',
        cor: 'blue',
        perfis: [...RECEPCAO, 'gerente', 'medico', 'tecnico', 'enfermagem'],
      },
      { id: 'agendar', rotulo: 'Novo agendamento', href: '/agendamentos/novo', icone: 'agendar', cor: 'blue', perfis: [...RECEPCAO, 'gerente'] },
      { id: 'recepcao', rotulo: 'Recepção e pulseiras', href: '/recepcao', icone: 'recepcao', cor: 'teal', perfis: RECEPCAO },
    ],
  },
  {
    id: 'exames',
    titulo: 'Exames',
    itens: [
      { id: 'sala', rotulo: 'Sala de exame', href: '/sala', icone: 'sala', cor: 'teal', perfis: ['tecnico', 'enfermagem', 'medico'] },
      { id: 'laudos', rotulo: 'Laudar', href: '/laudos', icone: 'laudos', cor: 'violet', perfis: ['medico'] },
      { id: 'estudos', rotulo: 'Conciliar estudos', href: '/estudos', icone: 'imagem', cor: 'indigo', perfis: ['medico'] },
    ],
  },
  {
    id: 'gestao',
    titulo: 'Gestão',
    itens: [
      { id: 'caixa', rotulo: 'Caixa (particular)', href: '/caixa', icone: 'caixa', cor: 'green', perfis: ['caixa'] },
      { id: 'autorizar-desconto', rotulo: 'Autorizar desconto', href: '/caixa/autorizacao-desconto', icone: 'autorizacao', cor: 'green', perfis: ['gerente'] },
      { id: 'estornos', rotulo: 'Aprovar estorno', href: '/caixa/estornos', icone: 'estorno', cor: 'green', perfis: ['gerente', 'financeiro'] },
    ],
  },
  {
    id: 'qualidade',
    titulo: 'Qualidade',
    itens: [
      { id: 'contingencia', rotulo: 'Contingência', href: '/contingencia', icone: 'contingencia', cor: 'rose', perfis: [...RECEPCAO, 'gerente'] },
    ],
  },
];

export function secoesDoMenu(perfis: readonly string[]): SecaoDoMenu[] {
  return SECOES.map((s) => ({
    id: s.id,
    titulo: s.titulo,
    itens: s.itens
      .filter((i) => i.perfis.some((p) => perfis.includes(p)))
      .map(({ id, rotulo, href, icone, cor }) => ({ id, rotulo, href, icone, cor })),
  })).filter((s) => s.itens.length > 0);
}

/** Item ativo pelo caminho (o mais específico ganha). */
export function itemAtivo(caminho: string, secoes: SecaoDoMenu[]): string {
  let melhor = '';
  let tamanho = 0;
  for (const s of secoes) {
    for (const i of s.itens) {
      if ((caminho === i.href || caminho.startsWith(`${i.href}/`)) && i.href.length > tamanho) {
        melhor = i.id;
        tamanho = i.href.length;
      }
    }
  }
  if (melhor === '' && caminho.startsWith('/triagem/')) {
    return 'sala';
  }
  return melhor;
}

/** Primeira tela do perfil (para onde `/` leva). */
export function telaInicial(perfis: readonly string[]): string {
  return secoesDoMenu(perfis)[0]?.itens[0]?.href ?? '/agenda';
}
