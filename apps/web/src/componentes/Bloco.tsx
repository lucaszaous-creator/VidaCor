import { Botao, Carregando, ErroTela, SemConexao, SemPermissao } from '@vidacor/design-system';
import type { ReactNode } from 'react';
import { erroParaTela } from '../erros';

export interface PropsDoEstadoDaConsulta {
  /** O que carrega ("a fila da sala"), para leitor de tela e mensagens. */
  oQue: string;
  carregando: boolean;
  erro: unknown;
  aoTentarDeNovo: () => void;
  /** Título e orientação quando o perfil não pode ver (sem revelar se o objeto existe). */
  semPermissao?: { titulo: string; detalhe: string; acao?: ReactNode };
  linhas?: number;
}

/**
 * Estado de um bloco de dados enquanto não há dado (Estados.dc.html): carregando, sem conexão, sem
 * permissão ou erro com a mensagem do servidor. Devolve `null` quando há dado para mostrar.
 */
export function EstadoDaConsulta({ oQue, carregando, erro, aoTentarDeNovo, semPermissao, linhas = 3 }: PropsDoEstadoDaConsulta) {
  if (erro !== null && erro !== undefined) {
    const e = erroParaTela(erro);
    const tentar = (
      <Botao pequeno onPress={aoTentarDeNovo}>
        Tentar de novo
      </Botao>
    );
    if (e.tipo === 'sem-permissao' && semPermissao !== undefined) {
      return <SemPermissao titulo={semPermissao.titulo} detalhe={semPermissao.detalhe} acao={semPermissao.acao} />;
    }
    if (e.tipo === 'sem-conexao') {
      return <SemConexao acao={tentar}>{`Não foi possível carregar ${oQue}. Nada foi alterado.`}</SemConexao>;
    }
    return <ErroTela mensagem={e.mensagem} codigoDeSuporte={e.codigoDeSuporte} acao={tentar} />;
  }
  if (carregando) {
    return <Carregando rotulo={`Carregando ${oQue}`} linhas={linhas} />;
  }
  return null;
}

/** Cabeçalho de uma linha das telas de sistema (título, complemento, ações à direita). */
export function CabecalhoDaTela({ titulo, complemento, children }: { titulo: string; complemento?: ReactNode; children?: ReactNode }) {
  return (
    <header className="row wrap" style={{ gap: 14, minHeight: 44 }}>
      <h1 style={{ margin: 0, fontSize: 24, fontWeight: 650, letterSpacing: '-.02em' }}>{titulo}</h1>
      {complemento}
      <div className="grow" />
      {children}
    </header>
  );
}
