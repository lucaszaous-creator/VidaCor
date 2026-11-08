/**
 * Documentos em PDF simulados para os testes de TELA (regras reais em
 * `crates/api/tests/documentos_pdf.rs`, com PostgreSQL real): laudo impresso, laudo no portal e
 * dossiê do lastro. O corpo é só o começo de um PDF; o que importa aqui é o que a tela faz.
 */
import { http, HttpResponse } from 'msw';
import { LAUDO_ANTONIO_ID, PORTAL_LAUDO_DOPPLER, PORTAL_LAUDO_ECO } from './dados/fase1';
import { estado } from './nucleo-simulado';
import { fase1, registrar, semPermissao, temPerfil } from './nucleo-fase1';
import { problema } from './problema';

export const SHA_FICTICIO = '9f2c4e7a1b3d5f60718293a4b5c6d7e8f9012345a6b7c8d9e0f1a2b3c4d5e6f7';

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

export const manipuladoresDocumentos = [
  http.get('*/v1/laudos/:id/pdf', ({ params }) => {
    if (!temPerfil('medico')) return semPermissao('ler laudos');
    if (estado.sessao.usuario.nome === 'Dr. Marcos Freire' && !fase1.acessoDeEmergencia) {
      return problema(403, 'sem-vinculo-de-cuidado', 'Você não tem vínculo de cuidado com este paciente. O laudo não foi aberto.');
    }
    if (params.id !== LAUDO_ANTONIO_ID) return problema(404, 'laudo-nao-encontrado', 'Este laudo não existe nesta unidade. Nada foi aberto.');
    const vigente = fase1.laudo.versoes.find((v) => v.situacao === 'assinada')?.numero;
    if (vigente === undefined) {
      return problema(
        409,
        'laudo-nao-assinado',
        'Este laudo ainda não foi assinado e não pode ser impresso. Nenhum PDF foi gerado. A impressão fica disponível depois da assinatura.',
      );
    }
    return pdf(`${fase1.laudo.codigo}-v${String(vigente)}.pdf`, 'DOC-2026-000007');
  }),
  http.get('*/v1/portal/laudos/:id/pdf', ({ params }) => {
    const laudo = [PORTAL_LAUDO_ECO, PORTAL_LAUDO_DOPPLER].find((l) => l.id === params.id);
    if (laudo === undefined) {
      return problema(404, 'nao-encontrado', 'Não encontramos este documento. Confira o endereço ou volte à lista dos seus exames.');
    }
    return pdf('LAU-2026-000031-v2.pdf', 'DOC-2026-000008');
  }),
  http.post('*/v1/lastro/dossie', async ({ request }) => {
    const corpo = (await registrar('dossie', request)) as { codigo: string };
    if (request.headers.get('X-CSRF-Token') !== estado.sessao.csrf) {
      return problema(403, 'csrf-invalido', 'Pedido sem a confirmação de segurança. Recarregue a página.');
    }
    if (!temPerfil('gerente', 'auditor')) return semPermissao('emitir o dossiê');
    if (!corpo.codigo.startsWith('ATD') && !corpo.codigo.startsWith('ACC')) {
      return problema(404, 'codigo-sem-cadeia', 'Nenhum atendimento desta unidade tem este código.');
    }
    return pdf('DOS-2026-000003.pdf', 'DOS-2026-000003');
  }),
];
