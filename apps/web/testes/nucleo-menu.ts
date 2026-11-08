/** Núcleo simulado do menu de contexto (docs/19 § 8). As regras reais estão em `crates/api/src/rotas/menu.rs`. */
import type { MenuDoObjeto } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { temPerfil } from './nucleo-fase1';
import { estado } from './nucleo-simulado';
import { problema } from './problema';

export const manipuladoresMenu = [
  http.get('*/v1/menu/:objeto/:id', ({ params }) => {
    const item = estado.itens.find((i) => i.id === params.id);
    if (item === undefined) return problema(404, 'agendamento-nao-encontrado', 'Agendamento não encontrado.');
    const menu: MenuDoObjeto = {
      cabecalho: { titulo: item.paciente.nome, subtitulo: `Nascimento ${item.paciente.nascimento.split('-').reverse().join('/')}, ${item.exame.nome}` },
      acoes: [
        ...(temPerfil('recepcao') ? [{ id: 'checkin', rotulo: 'Fazer check-in', estado: 'disponivel' as const, perigo: false, destino: `/recepcao?agendamento=${item.id}` }] : []),
        { id: 'copiar', rotulo: 'Copiar nome e nascimento', estado: 'disponivel' as const, perigo: false, destino: null },
        ...(temPerfil('recepcao')
          ? [{ id: 'reimprimir', rotulo: 'Reimprimir pulseira', estado: 'indisponivel' as const, motivo: 'Faça o check-in antes', perigo: true, destino: null }]
          : []),
      ],
    };
    return HttpResponse.json(menu);
  }),
];
