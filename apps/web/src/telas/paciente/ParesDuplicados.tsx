import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { CadastroDoPar, DecisaoSobreDuplicado, ParDuplicado } from '@vidacor/api-client';
import { Aviso, Botao, Campo, formatarData, formatarDataHora, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

function Cadastro({ c }: { c: CadastroDoPar }) {
  return (
    <div style={{ minWidth: 0 }}>
      <b style={{ display: 'block' }}>{c.nome}</b>
      <span className="t13 mute num" style={{ display: 'block' }}>
        {formatarData(c.nascimento)}, prontuário {c.codigo}
      </span>
      <span className="t13 mute num" style={{ display: 'block' }}>
        CPF {c.cpf_mascarado ?? 'não informado'}, celular {c.celular ?? 'não informado'}
      </span>
    </div>
  );
}

function Par({ p }: { p: ParDuplicado }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((d: DecisaoSobreDuplicado, chave: string) => api.pacientes.decidirDuplicado(d, { idempotencyKey: chave }), {
    aoConcluir: async () => {
      await cliente.invalidateQueries({ queryKey: ['pares-duplicados'] });
      await cliente.invalidateQueries({ queryKey: ['pacientes-lista'] });
    },
  });
  const decidir = (decisao: DecisaoSobreDuplicado['decisao'], comMotivo: boolean) => {
    op.enviar({ paciente_a: p.a.id, paciente_b: p.b.id, decisao, motivo: comMotivo ? motivo.trim() : null });
  };
  // A resposta do servidor vale na hora (a tela nunca mente), sem esperar a lista recarregar.
  const feito = op.resultado;
  return (
    <article className="card" style={{ padding: 14, display: 'flex', flexDirection: 'column', gap: 10 }} aria-label={`Possível duplicado: ${p.a.nome}`}>
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 12 }}>
        <Cadastro c={p.a} />
        <Cadastro c={p.b} />
      </div>
      <span className="t13">{p.motivo}.</span>
      {feito !== null ? (
        <Aviso nivel="feito" titulo={feito.decisao === 'juncao_pedida' ? 'Junção pedida.' : 'Marcados como pessoas diferentes.'}>
          {feito.decisao === 'juncao_pedida'
            ? `Registro ${feito.codigo}. Aguarda a conferência de outra pessoa; até lá os cadastros seguem separados.`
            : `Registro ${feito.codigo} no lastro. O par sai desta lista.`}
        </Aviso>
      ) : p.situacao === 'juncao_pedida' ? (
        <>
          <p className="t13" style={{ margin: 0 }}>
            Junção pedida{p.decidido_por === null || p.decidido_por === undefined ? '' : ` por ${p.decidido_por}`}
            {p.decidido_em === null || p.decidido_em === undefined ? '' : ` em ${formatarDataHora(p.decidido_em, FUSO_DA_CLINICA)}`}. Aguarda a conferência de outra pessoa; até lá os cadastros seguem separados.
          </p>
          <Campo
            rotulo="Se forem pessoas diferentes, diga por quê"
            value={motivo}
            onChange={setMotivo}
            erro={tentou && motivo.trim().length < 5 ? 'Escreva o motivo (5 letras ou mais). A decisão anterior fica no histórico.' : null}
          />
          <div className="row">
            <Botao
              pequeno
              ocupado={op.enviando}
              onPress={() => {
                setTentou(true);
                if (motivo.trim().length >= 5) decidir('pessoas_diferentes', true);
              }}
            >
              São pessoas diferentes
            </Botao>
          </div>
        </>
      ) : (
        <div className="row wrap" style={{ gap: 8 }}>
          <Botao
            pequeno
            ocupado={op.enviando}
            onPress={() => {
              decidir('juncao_pedida', false);
            }}
          >
            Pedir junção
          </Botao>
          <Botao
            pequeno
            ocupado={op.enviando}
            onPress={() => {
              decidir('pessoas_diferentes', false);
            }}
          >
            São pessoas diferentes
          </Botao>
        </div>
      )}
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="registrar a decisão" />}
    </article>
  );
}

/** Aba "Possíveis duplicados" da lista de pacientes (recepção e gerência). */
export function ParesDuplicados() {
  const { api } = useDependencias();
  const pares = useQuery({ queryKey: ['pares-duplicados'], queryFn: () => api.pacientes.paresDuplicados() });
  return (
    <div className="col" style={{ gap: 12 }}>
      <EstadoDaConsulta oQue="os possíveis duplicados" carregando={pares.isPending} erro={pares.error} aoTentarDeNovo={() => void pares.refetch()} />
      {pares.data?.length === 0 && <Vazio titulo="Nenhum possível duplicado." detalhe="Pares conferidos como pessoas diferentes não voltam para esta lista." />}
      {pares.data?.map((p) => (
        <Par key={`${p.a.id}-${p.b.id}`} p={p} />
      ))}
    </div>
  );
}
