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
      // Início (Main.dc.html): recepção e gerência entram por aqui.
      { id: 'inicio', rotulo: 'Início', href: '/inicio', icone: 'agenda', cor: 'blue', perfis: [...RECEPCAO, 'gerente'] },
      {
        id: 'agenda',
        rotulo: 'Agenda e check-in',
        href: '/agenda',
        icone: 'agenda',
        cor: 'blue',
        perfis: [...RECEPCAO, 'gerente', 'medico', 'tecnico', 'enfermagem'],
      },
      { id: 'agendar', rotulo: 'Novo agendamento', href: '/agendamentos/novo', icone: 'agendar', cor: 'blue', perfis: [...RECEPCAO, 'gerente'] },
      { id: 'paciente', rotulo: 'Pacientes', href: '/pacientes', icone: 'recepcao', cor: 'blue', perfis: [...RECEPCAO, 'gerente', 'medico', 'tecnico'] },
      { id: 'lista-espera', rotulo: 'Lista de espera', href: '/lista-espera', icone: 'agenda', cor: 'blue', perfis: [...RECEPCAO, 'gerente'] },
      { id: 'orcamento', rotulo: 'Orçamento particular', href: '/orcamentos', icone: 'caixa', cor: 'blue', perfis: [...RECEPCAO, 'gerente'] },
      { id: 'exame-externo', rotulo: 'Exame externo', href: '/exames-externos', icone: 'imagem', cor: 'blue', perfis: [...RECEPCAO, 'gerente'] },
      { id: 'novo-paciente', rotulo: 'Novo paciente', href: '/pacientes/novo', icone: 'recepcao', cor: 'blue', perfis: [...RECEPCAO, 'gerente'] },
      { id: 'recepcao', rotulo: 'Recepção e pulseiras', href: '/recepcao', icone: 'recepcao', cor: 'teal', perfis: RECEPCAO },
      // A conta da TV só tem este item, e `/` a leva direto ao painel.
      { id: 'painel', rotulo: 'TV de chamada', href: '/telao', icone: 'recepcao', cor: 'teal', perfis: ['tv', ...RECEPCAO, 'gerente'] },
      { id: 'autorizacoes', rotulo: 'Autorizações', href: '/autorizacoes', icone: 'autorizacao', cor: 'blue', perfis: [...RECEPCAO, 'faturista', 'gerente'] },
    ],
  },
  {
    id: 'exames',
    titulo: 'Exames',
    itens: [
      { id: 'sala', rotulo: 'Sala de exame', href: '/sala', icone: 'sala', cor: 'teal', perfis: ['tecnico', 'enfermagem', 'medico'] },
      { id: 'laudos', rotulo: 'Laudar', href: '/laudos', icone: 'laudos', cor: 'violet', perfis: ['medico'] },
      { id: 'fluxo-laudos', rotulo: 'Fluxo de laudos', href: '/fluxo-de-laudos', icone: 'laudos', cor: 'violet', perfis: ['gerente'] },
      { id: 'estudos', rotulo: 'Conciliar estudos', href: '/estudos', icone: 'imagem', cor: 'indigo', perfis: ['medico'] },
    ],
  },
  {
    id: 'gestao',
    titulo: 'Gestão',
    itens: [
      { id: 'caixa', rotulo: 'Caixa (particular)', href: '/caixa', icone: 'caixa', cor: 'green', perfis: ['caixa'] },
      { id: 'financeiro', rotulo: 'Financeiro', href: '/financeiro', icone: 'caixa', cor: 'green', perfis: ['financeiro', 'gerente'] },
      { id: 'indicadores', rotulo: 'Indicadores', href: '/indicadores', icone: 'agenda', cor: 'indigo', perfis: ['gerente', 'financeiro'] },
      { id: 'estoque', rotulo: 'Estoque', href: '/estoque', icone: 'etiquetas', cor: 'green', perfis: ['estoque', 'gerente'] },
      { id: 'pendencias', rotulo: 'Pendências do caixa', href: '/caixa/pendencias', icone: 'autorizacao', cor: 'green', perfis: ['gerente', 'financeiro'] },
      { id: 'faturamento', rotulo: 'Faturamento TISS', href: '/faturamento', icone: 'caixa', cor: 'green', perfis: ['faturista', 'gerente'] },
      { id: 'auditoria-faturamento', rotulo: 'Auditoria do faturamento', href: '/faturamento/auditoria', icone: 'confere', cor: 'green', perfis: ['faturista', 'gerente'] },
    ],
  },
  {
    id: 'qualidade',
    titulo: 'Qualidade',
    itens: [
      { id: 'configuracoes', rotulo: 'Configurações', href: '/configuracoes', icone: 'cadeado', cor: 'rose', perfis: ['ti', 'gerente'] },
      { id: 'aparelhos', rotulo: 'Aparelhos', href: '/aparelhos', icone: 'sala', cor: 'rose', perfis: ['ti', 'gerente'] },
      { id: 'usuarios', rotulo: 'Usuários e perfis', href: '/usuarios', icone: 'cadeado', cor: 'rose', perfis: ['ti', 'gerente'] },
      // Cadastros base (docs/04): corpo clínico só da gerência; o catálogo a recepção consulta.
      { id: 'corpo-clinico', rotulo: 'Corpo clínico', href: '/corpo-clinico', icone: 'laudos', cor: 'rose', perfis: ['gerente'] },
      { id: 'exames', rotulo: 'Exames e procedimentos', href: '/exames', icone: 'etiquetas', cor: 'rose', perfis: [...RECEPCAO, 'gerente'] },
      // Qualidade e segurança do paciente: toda a equipe notifica (menos a TV).
      { id: 'qualidade', rotulo: 'Qualidade', href: '/qualidade', icone: 'confere', cor: 'rose', perfis: ['recepcao', 'supervisor_recepcao', 'coordenacao', 'caixa', 'tecnico', 'enfermagem', 'medico', 'faturista', 'financeiro', 'gerente', 'estoque', 'auditor', 'ti'] },
      { id: 'ouvidoria', rotulo: 'Ouvidoria', href: '/ouvidoria', icone: 'confere', cor: 'rose', perfis: [...RECEPCAO, 'gerente'] },
      { id: 'lastro', rotulo: 'Lastro', href: '/lastro', icone: 'busca', cor: 'rose', perfis: ['gerente', 'auditor'] },
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
