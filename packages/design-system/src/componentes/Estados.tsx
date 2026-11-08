import type { ReactNode } from 'react';
import { Aviso } from './Aviso';
import { Icone } from './Icone';

/**
 * Estados de tela (Estados.dc.html). Todo bloco de dados mostra um deles enquanto não há dado:
 * a tela nunca mostra "0" ou lista vazia no lugar de erro (R8).
 */

export interface PropsDoVazio {
  titulo: string;
  detalhe?: string;
  acao?: ReactNode;
}

export function Vazio({ titulo, detalhe, acao }: PropsDoVazio) {
  return (
    <div className="state state-box">
      <div className="b">{titulo}</div>
      {detalhe !== undefined && <div className="t14 mute">{detalhe}</div>}
      {acao}
    </div>
  );
}

export interface PropsDoCarregando {
  /** O que está carregando, para o leitor de tela ("Carregando a agenda"). */
  rotulo: string;
  linhas?: number;
}

/** Esqueleto com a forma do conteúdo (único gradiente permitido, parado com movimento reduzido). */
export function Carregando({ rotulo, linhas = 3 }: PropsDoCarregando) {
  const larguras = ['60%', '70%', '55%', '65%', '50%'];
  return (
    <div className="col" style={{ gap: 14 }} role="status" aria-label={rotulo}>
      {Array.from({ length: linhas }, (_, i) => (
        <div className="sk-linha" key={i}>
          <div className="sk" style={{ width: 40, height: 40, borderRadius: '50%' }} />
          <div className="col" style={{ gap: 6, flex: 1 }}>
            <div className="sk" style={{ height: 12, width: larguras[i % larguras.length] }} />
            <div className="sk" style={{ height: 10, width: '40%' }} />
          </div>
        </div>
      ))}
    </div>
  );
}

export interface PropsDoSemConexao {
  /** Desde quando, se conhecido ("10:12"). */
  desde?: string;
  /** O que continua funcionando e o que fazer (R7, R8). */
  children?: ReactNode;
  acao?: ReactNode;
}

export function SemConexao({ desde, children, acao }: PropsDoSemConexao) {
  return (
    <div className="banner off" role="status">
      <Icone nome="semConexao" tamanho={20} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div className="t14">
          <b>Sem conexão com o servidor{desde === undefined ? '.' : ` desde ${desde}.`}</b> {children}
        </div>
        {acao !== undefined && <div style={{ marginTop: 10 }}>{acao}</div>}
      </div>
    </div>
  );
}

export interface PropsDoErroTela {
  /** `detail` do Problem Details, já no formato R8 (o que aconteceu, o que foi feito, o que fazer). */
  mensagem: string;
  /** `trace_id`, mostrado como código de suporte. */
  codigoDeSuporte?: string;
  acao?: ReactNode;
}

export function ErroTela({ mensagem, codigoDeSuporte, acao }: PropsDoErroTela) {
  return (
    <Aviso nivel="bloqueante" codigoDeSuporte={codigoDeSuporte} acoes={acao}>
      {mensagem}
    </Aviso>
  );
}

export interface PropsDoSemPermissao {
  titulo: string;
  /** A quem pedir. Não revela se o objeto existe (SEC-DAD-4). */
  detalhe: string;
  acao?: ReactNode;
}

export function SemPermissao({ titulo, detalhe, acao }: PropsDoSemPermissao) {
  return (
    <div className="state state-box" role="status">
      <Icone nome="cadeado" tamanho={28} traco={1.8} />
      <div className="b">{titulo}</div>
      <div className="t14 mute" style={{ maxWidth: 360 }}>
        {detalhe}
      </div>
      {acao}
    </div>
  );
}
