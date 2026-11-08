import { ErroDaApi, ErroSemConexao } from '@vidacor/api-client';

/** O que a tela mostra para um erro: sempre o texto do servidor quando há (R8), nunca inventado. */
export interface ErroParaTela {
  tipo: 'sem-sessao' | 'sem-permissao' | 'sem-conexao' | 'servidor';
  mensagem: string;
  codigoDeSuporte?: string;
  codigo?: string;
}

export function erroParaTela(erro: unknown): ErroParaTela {
  if (erro instanceof ErroDaApi) {
    const base = { mensagem: erro.problema.detail, codigoDeSuporte: erro.traceId, codigo: erro.codigo };
    if (erro.status === 401) {
      return { tipo: 'sem-sessao', ...base };
    }
    if (erro.status === 403) {
      return { tipo: 'sem-permissao', ...base };
    }
    return { tipo: 'servidor', ...base };
  }
  if (erro instanceof ErroSemConexao) {
    return { tipo: 'sem-conexao', mensagem: 'Não foi possível falar com o servidor.' };
  }
  return {
    tipo: 'servidor',
    mensagem: 'Algo deu errado nesta tela. Nada foi confirmado. Recarregue a página; se continuar, avise a TI.',
  };
}

/** Leitura não tenta de novo quando o servidor recusou (4xx): repetir não muda a resposta. */
export function deveTentarDeNovo(tentativas: number, erro: unknown): boolean {
  if (erro instanceof ErroDaApi && erro.status < 500) {
    return false;
  }
  return tentativas < 2;
}
