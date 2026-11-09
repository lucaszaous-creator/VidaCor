import { useQueryClient } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { LinhaDeEtiqueta } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Escolha, EtiquetaPulseira, formatarData } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { BotaoImprimir } from '../recepcao/AcoesDaPulseira';

const MOTIVOS_DE_REIMPRESSAO = [
  { valor: 'rasgou', rotulo: 'Rasgou ou apagou' },
  { valor: 'dado impresso errado', rotulo: 'Dado impresso errado' },
  { valor: 'paciente perdeu', rotulo: 'Paciente perdeu' },
];

type Acao = 'nenhuma' | 'reimprimir' | 'invalidar';

/**
 * Reimprimir (recepção: a anterior deixa de valer) e invalidar com motivo (sem nova etiqueta). Os dois
 * pedem o motivo antes; a etiqueta antiga, se bipada depois, é bloqueada pela sala (docs/20).
 */
export function AcoesDaEtiqueta({ e, recepcao }: { e: LinhaDeEtiqueta; recepcao: boolean }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [acao, setAcao] = useState<Acao>('nenhuma');
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const recarregar = async () => {
    await cliente.invalidateQueries({ queryKey: ['etiquetas'] });
  };
  const reimp = useOperacao((m: string, chave: string) => api.etiquetas.reimprimir(e.codigo, { motivo: m }, { idempotencyKey: chave }), { aoConcluir: recarregar });
  const inval = useOperacao((m: string, chave: string) => api.etiquetas.invalidar(e.codigo, m, { idempotencyKey: chave }), { aoConcluir: recarregar });
  const nasc = formatarData(e.paciente.nascimento);

  if (reimp.resultado !== null) {
    const r = reimp.resultado;
    return (
      <div className="col" style={{ gap: 10 }}>
        <Aviso nivel="feito" titulo={`Nova pulseira ${r.nova.codigo} pronta para imprimir.`} acoes={<BotaoImprimir />}>
          A {r.invalidada.codigo} deixou de valer. Confira nome e nascimento com o paciente ao colocar a nova.
        </Aviso>
        <div className="etq-impressao">
          <EtiquetaPulseira nome={e.paciente.nome} nascimento={nasc} codigo={r.nova.codigo} accession={e.accession} situacao="valida" />
        </div>
      </div>
    );
  }
  if (inval.resultado !== null) {
    return (
      <Aviso nivel="feito" titulo={`Etiqueta ${inval.resultado.codigo} invalidada.`}>
        Motivo: {inval.resultado.motivo_invalidacao}. Se for bipada, a sala bloqueia o exame e avisa.
      </Aviso>
    );
  }
  const valida = e.estado !== 'invalidada';
  if (acao === 'nenhuma') {
    return (
      <div className="row wrap" style={{ gap: 8 }}>
        {recepcao && valida && (
          <>
            <Botao
              pequeno
              onPress={() => {
                setAcao('reimprimir');
                setMotivo('');
                setTentou(false);
              }}
            >
              Reimprimir
            </Botao>
            <Botao
              pequeno
              onPress={() => {
                setAcao('invalidar');
                setMotivo('');
                setTentou(false);
              }}
            >
              Invalidar
            </Botao>
          </>
        )}
        <Link to="/pacientes" search={{ id: e.paciente.id }} className="btn btn-sm">
          Abrir ficha
        </Link>
      </div>
    );
  }
  const op = acao === 'reimprimir' ? reimp : inval;
  const ok = acao === 'reimprimir' ? motivo !== '' : motivo.trim().length >= 3;
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label={acao === 'reimprimir' ? `Reimprimir ${e.codigo}` : `Invalidar ${e.codigo}`}
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (ok) op.enviar(motivo.trim());
      }}
    >
      <b className="t14">
        {acao === 'reimprimir' ? 'Reimprimir' : 'Invalidar'} a pulseira de {e.paciente.nome}, {nasc}
      </b>
      {acao === 'reimprimir' ? (
        <Escolha rotulo="Motivo" valor={motivo} aoMudar={setMotivo} opcoes={MOTIVOS_DE_REIMPRESSAO} vazio="Escolha o motivo" />
      ) : (
        <Campo rotulo="Motivo" value={motivo} onChange={setMotivo} ajuda="Por exemplo: paciente desistiu do exame." />
      )}
      {tentou && !ok && <span className="campo-erro">Escolha ou escreva o motivo.</span>}
      <p className="t13 mute" style={{ margin: 0 }}>
        A etiqueta atual deixa de valer. Se for bipada depois, o sistema bloqueia e avisa.
      </p>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato={acao === 'reimprimir' ? 'reimprimir a etiqueta' : 'invalidar a etiqueta'} />}
      <div className="row" style={{ gap: 8 }}>
        <Botao
          pequeno
          onPress={() => {
            setAcao('nenhuma');
          }}
        >
          Voltar
        </Botao>
        <Botao pequeno type="submit" variante="principal" ocupado={op.enviando}>
          Confirmar
        </Botao>
      </div>
    </form>
  );
}
