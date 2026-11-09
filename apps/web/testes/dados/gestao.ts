/**
 * Dados fictícios da minha agenda do médico, gestão de dose e relatórios, usados pelos testes de tela
 * (MSW) e pela verificação "a tela cabe na janela" (Playwright).
 */
import type { AusenciaDoMedico, ExamesComDose, MinhaAgenda, RelatorioDoCatalogo } from '@vidacor/api-client';

export const HOJE = '2026-11-06';

export function agenda(): MinhaAgenda {
  return {
    data: HOJE,
    medico: 'Dr. Paulo Medeiros',
    turnos: [{ sala: 'Sala 1', inicio: '2026-11-06T10:00:00Z', fim: '2026-11-06T15:00:00Z' }],
    pacientes: 3,
    ja_atendidos: 1,
    atendidos_com_laudo_pendente: 1,
    esperando: 1,
    laudos_para_assinar: 7,
    laudos_vencem_hoje: 2,
    itens: [
      {
        agendamento_id: '0000000d-0000-7000-8000-000000000001',
        codigo: 'AGD-2026-000101',
        inicio: '2026-11-06T10:00:00Z',
        paciente: { id: '0000000e-0000-7000-8000-000000000001', nome: 'Eduardo Melo Vaz', nascimento: '1977-09-21' },
        exame: 'Teste ergométrico',
        sala: 'Sala 1',
        grupo: 'liberado',
        atendimento_id: '0000000f-0000-7000-8000-000000000001',
        laudo_situacao: 'rascunho',
        laudo_id: '00000010-0000-7000-8000-000000000001',
      },
      {
        agendamento_id: '0000000d-0000-7000-8000-000000000002',
        codigo: 'AGD-2026-000102',
        inicio: '2026-11-06T11:30:00Z',
        paciente: { id: '0000000e-0000-7000-8000-000000000002', nome: 'Antônio Carlos Ribeiro', nascimento: '1954-06-15' },
        exame: 'Eco e Doppler de carótidas',
        sala: 'Sala 1',
        grupo: 'esperando',
        atendimento_id: '0000000f-0000-7000-8000-000000000002',
        esperando_desde: new Date(Date.now() - 19 * 60_000).toISOString(),
        senha: 'P012',
      },
      {
        agendamento_id: '0000000d-0000-7000-8000-000000000003',
        codigo: 'AGD-2026-000103',
        inicio: '2026-11-06T12:00:00Z',
        paciente: { id: '0000000e-0000-7000-8000-000000000003', nome: 'Cláudia Bastos Nunes', nascimento: '1965-08-22' },
        exame: 'Eco com Doppler',
        sala: 'Sala 1',
        grupo: 'a_chegar',
      },
    ],
    avisos: [{ tipo: 'imagens_recebidas', texto: 'Imagens de Sônia Maria Castro chegaram (24 imagens)', quando: '2026-11-06T11:47:00Z' }],
    semana: [
      { data: '2026-11-02', pacientes: 12 },
      { data: '2026-11-03', pacientes: 15 },
      { data: '2026-11-04', pacientes: 0, ausencia: 'Congresso ou curso' },
      { data: '2026-11-05', pacientes: 13 },
      { data: '2026-11-06', pacientes: 14 },
      { data: '2026-11-07', pacientes: 0 },
    ],
  };
}

export function ausenciaAguardando(): AusenciaDoMedico {
  return {
    id: '00000011-0000-7000-8000-000000000001',
    codigo: 'AUS-2026-000042',
    medico_id: '0000000a-0000-7000-8000-000000000004',
    medico: 'Dr. Paulo Medeiros',
    inicio: '2026-11-20T03:00:00Z',
    fim: '2026-11-23T03:00:00Z',
    motivo: 'congresso',
    detalhe: 'Congresso de cardiologia',
    afetados_no_pedido: 14,
    situacao: 'aguardando',
    criado_em: '2026-11-06T11:41:00Z',
  };
}

export function doses(): ExamesComDose {
  return {
    de: '2026-11-01',
    ate: '2026-11-07',
    acima_do_nrd: 1,
    sem_justificativa: 1,
    exames: [
      {
        dose_id: '00000012-0000-7000-8000-000000000001',
        codigo: 'DOS-2026-000001',
        paciente: { id: '0000000e-0000-7000-8000-000000000002', nome: 'Antônio Carlos Ribeiro', nascimento: '1954-06-15' },
        menor_de_idade: false,
        exame: 'TC de tórax',
        modalidade: 'CT',
        aparelho: 'Somatom go.Up',
        realizado_em: '2026-11-07T12:12:00Z',
        ctdivol_mgy: 14.2,
        dlp_mgycm: 520,
        nrd: { grandeza: 'ctdivol', valor: 12 },
        valor_comparado: 14.2,
        acima_do_nrd: true,
      },
      {
        dose_id: '00000012-0000-7000-8000-000000000002',
        codigo: 'DOS-2026-000002',
        paciente: { id: '0000000e-0000-7000-8000-000000000004', nome: 'Lúcia Fernandes Prado', nascimento: '1959-07-02' },
        menor_de_idade: false,
        exame: 'TC de abdome sem contraste',
        modalidade: 'CT',
        aparelho: 'Somatom go.Up',
        realizado_em: '2026-11-06T18:40:00Z',
        ctdivol_mgy: 9.8,
        dlp_mgycm: 470,
        nrd: { grandeza: 'ctdivol', valor: 15 },
        valor_comparado: 9.8,
        acima_do_nrd: false,
      },
    ],
  };
}

export const CATALOGO: (RelatorioDoCatalogo & { perfis: string[] })[] = [
  { id: 'producao_medico', grupo: 'Produção', titulo: 'Produção por médico', descricao: 'Laudos assinados por médico no período', dado_de_saude: false, perfis: ['gerente'] },
  { id: 'producao_convenio', grupo: 'Produção', titulo: 'Produção por convênio', descricao: 'Guias e valor por operadora', dado_de_saude: false, perfis: ['gerente', 'faturista'] },
  { id: 'faltas', grupo: 'Agenda', titulo: 'Faltas', descricao: 'Pacientes que faltaram, por código', dado_de_saude: true, perfis: ['gerente'] },
  { id: 'glosas', grupo: 'Faturamento', titulo: 'Glosas', descricao: 'Valor glosado por convênio', dado_de_saude: false, perfis: ['gerente', 'faturista'] },
];

