import type { EfeitoDaPendencia, SituacaoDaConta, SituacaoDaGlosa, SituacaoDoLote } from '@vidacor/api-client';
import type { CorDoChip } from '@vidacor/design-system';

/** Textos e cores das situações vindas do servidor (a tela só mostra; quem decide é o núcleo). */
export const SITUACAO_DA_CONTA: Record<SituacaoDaConta, { texto: string; cor: CorDoChip }> = {
  em_conferencia: { texto: 'Em conferência', cor: 'cinza' },
  pronta: { texto: 'Pronta para o lote', cor: 'verde' },
  com_pendencia: { texto: 'Com pendência', cor: 'ambar' },
  em_lote: { texto: 'Em lote', cor: 'azul' },
  enviada: { texto: 'Enviada', cor: 'azul' },
  paga: { texto: 'Paga', cor: 'verde' },
  paga_com_glosa: { texto: 'Paga com glosa', cor: 'ambar' },
  glosada: { texto: 'Glosada', cor: 'rosa' },
};

export const EFEITO: Record<EfeitoDaPendencia, { texto: string; cor: CorDoChip }> = {
  bloqueia: { texto: 'Bloqueia', cor: 'rosa' },
  corrigir_antes: { texto: 'Corrigir antes', cor: 'ambar' },
  atencao: { texto: 'Atenção', cor: 'azul' },
};

export const SITUACAO_DO_LOTE: Record<SituacaoDoLote, { texto: string; cor: CorDoChip }> = {
  gerado: { texto: 'Gerado, não enviado', cor: 'cinza' },
  aguardando_envio: { texto: 'Aguardando envio', cor: 'ambar' },
  enviado: { texto: 'Enviado, aguardando retorno', cor: 'azul' },
  demonstrativo_recebido: { texto: 'Retorno recebido', cor: 'verde' },
};

export const SITUACAO_DA_GLOSA: Record<SituacaoDaGlosa, { texto: string; cor: CorDoChip }> = {
  aberta: { texto: 'Aberta', cor: 'rosa' },
  em_recurso: { texto: 'Em recurso', cor: 'azul' },
};

/** Conselho e UF no código do TISS, como o servidor exige (06 = CRM; 33 = RJ). */
export const CONSELHOS = [{ id: '06', rotulo: 'CRM (06)' }];
