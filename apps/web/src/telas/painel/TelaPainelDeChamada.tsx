import { useQuery } from '@tanstack/react-query';
import type { ChamadaNoPainel } from '@vidacor/api-client';
import { formatarHora } from '@vidacor/design-system';
import { useEffect, useState } from 'react';
import simbolo from '../../assets/simbolo.png';
import { useDependencias } from '../../contexto';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import './painel.css';

const INTERVALO_MS = 4_000;

function useAgora(): Date {
  const [agora, setAgora] = useState(() => new Date());
  useEffect(() => {
    const t = setInterval(() => {
      setAgora(new Date());
    }, 15_000);
    return () => {
      clearInterval(t);
    };
  }, []);
  return agora;
}

/**
 * TV de chamada (Telao.dc.html). Mostra só o que o servidor manda para o dispositivo TV: senha, nome
 * abreviado e destino (docs/18). Sem conexão, mantém a última chamada e diz desde quando está sem
 * atualizar (R: a tela nunca mente).
 */
export function TelaPainelDeChamada() {
  const { api } = useDependencias();
  const agora = useAgora();
  const painel = useQuery({ queryKey: ['painel-de-chamada'], queryFn: () => api.painelDeChamada(), refetchInterval: INTERVALO_MS, retry: false });
  const atual: ChamadaNoPainel | null = painel.data?.agora ?? null;
  const semConexao = painel.isError;
  const atualizadoEm = painel.dataUpdatedAt > 0 ? new Date(painel.dataUpdatedAt).toISOString() : null;
  return (
    <main className="painel" aria-label="Painel de chamada">
      <header>
        <div className="marca">
          <img src={simbolo} alt="" />
          VidaCor Imagem
        </div>
        <div className="relogio">
          <b className="num">{formatarHora(agora.toISOString(), FUSO_DA_CLINICA)}</b>
          <span className="claro">{agora.toLocaleDateString('pt-BR', { weekday: 'long', day: 'numeric', month: 'long', timeZone: FUSO_DA_CLINICA })}</span>
        </div>
      </header>
      <section className="atual" aria-label="Chamada atual" aria-live="assertive" aria-atomic="true">
        {semConexao ? (
          <div className="estado sem-conexao" role="status">
            <i />
            {atualizadoEm === null ? 'Painel sem conexão. As chamadas seguem em voz alta.' : `Sem atualizar desde ${formatarHora(atualizadoEm, FUSO_DA_CLINICA)}. As chamadas seguem em voz alta.`}
          </div>
        ) : (
          <div className="estado">
            <i />
            {atual === null ? 'Aguardando a primeira chamada' : atual.rechamada ? 'Chamando de novo' : 'Chamando agora'}
          </div>
        )}
        {atual !== null && (
          <>
            <div style={{ display: 'flex', alignItems: 'flex-end', gap: 36, minWidth: 0 }}>
              <div>
                <div className="rotulo">Senha</div>
                <div className="senha num">{atual.senha}</div>
              </div>
              <div style={{ paddingBottom: 18, minWidth: 0 }}>
                <div className="rotulo">Dirija-se ao local</div>
                <div className="destino">{atual.destino}</div>
              </div>
            </div>
            <div className="nome">{atual.nome_abreviado}</div>
          </>
        )}
      </section>
      <aside aria-label="Últimas chamadas">
        <div className="rotulo" style={{ fontSize: 20 }}>
          Últimas chamadas
        </div>
        <ol>
          {(painel.data?.ultimas ?? []).map((c) => (
            <li key={`${c.senha}-${c.chamada_em}`}>
              <b className="num">{c.senha}</b>
              <div>
                <div style={{ fontSize: 20, fontWeight: 600 }}>{c.destino}</div>
                <div className="claro" style={{ fontSize: 15 }}>
                  {formatarHora(c.chamada_em, FUSO_DA_CLINICA)}
                </div>
              </div>
            </li>
          ))}
        </ol>
      </aside>
      <footer>
        <span style={{ flex: 1 }}>
          Seu laudo chega pelo WhatsApp e fica no portal <b>Meu VidaCor</b>. Só mostramos as iniciais do seu nome no painel.
        </span>
      </footer>
    </main>
  );
}
