import type { Divergencia, PedidoNumeros } from '@vidacor/api-client';
import { AreaDeTexto, Aviso, Botao, Campo, DialogoDeFormulario, Escolha } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

const NUMERO = /^\d{1,4}$/;

/**
 * Números da faixa que voltaram sem uso ou foram anulados (docs/30 § 7): sem isso o encerramento
 * acusa "número sem situação". A faixa vale de `de` até `ate`, inclusive.
 */
export function NumerosSemUso({ contingenciaId }: { contingenciaId: string }) {
  const { api } = useDependencias();
  const [formulario, setFormulario] = useState('');
  const [de, setDe] = useState('');
  const [ate, setAte] = useState('');
  const [situacao, setSituacao] = useState('');
  const [tentou, setTentou] = useState(false);
  const [feito, setFeito] = useState<string | null>(null);
  const op = useOperacao((p: PedidoNumeros, chave: string) => api.contingencias.numeros(contingenciaId, p, { idempotencyKey: chave }), {
    aoConcluir: () => {
      setFeito(`${formulario} ${de.padStart(4, '0')} a ${ate.padStart(4, '0')}: ${situacao}.`);
      op.reiniciar();
      setTentou(false);
    },
  });

  function registrar(): void {
    setTentou(true);
    setFeito(null);
    if (formulario === '' || situacao === '' || !NUMERO.test(de.trim()) || !NUMERO.test(ate.trim())) return;
    op.enviar({
      formulario: formulario as PedidoNumeros['formulario'],
      de: Number(de.trim()),
      ate: Number(ate.trim()),
      situacao: situacao as PedidoNumeros['situacao'],
    });
  }

  return (
    <form
      className="col"
      style={{ gap: 10 }}
      noValidate
      aria-label="Números sem uso"
      onSubmit={(ev) => {
        ev.preventDefault();
        registrar();
      }}
    >
      <h3 className="t15 b">Números da faixa sem uso</h3>
      <div className="grid g2" style={{ gap: 10 }}>
        <Escolha
          rotulo="Formulário"
          valor={formulario}
          aoMudar={setFormulario}
          vazio="Escolha o formulário"
          opcoes={[
            { valor: 'F-ATD', rotulo: 'Ficha F-ATD' },
            { valor: 'F-REC', rotulo: 'Recibo F-REC' },
          ]}
          erro={tentou && formulario === '' ? 'Escolha o formulário.' : null}
        />
        <Escolha
          rotulo="Situação"
          valor={situacao}
          aoMudar={setSituacao}
          vazio="Escolha a situação"
          opcoes={[
            { valor: 'devolvido', rotulo: 'Devolvido sem uso' },
            { valor: 'anulado', rotulo: 'Anulado (rasurado ou perdido)' },
          ]}
          erro={tentou && situacao === '' ? 'Escolha a situação.' : null}
        />
        <Campo rotulo="Do número" value={de} onChange={setDe} inputMode="numeric" erro={tentou && !NUMERO.test(de.trim()) ? 'Digite o número inicial.' : null} />
        <Campo rotulo="Até o número" value={ate} onChange={setAte} inputMode="numeric" erro={tentou && !NUMERO.test(ate.trim()) ? 'Digite o número final.' : null} />
      </div>
      {feito !== null && (
        <Aviso nivel="feito" titulo="Números registrados.">
          {feito}
        </Aviso>
      )}
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="registrar os números" aoTentarDeNovo={registrar} />}
      <div className="row">
        <Botao type="submit" ocupado={op.enviando}>
          Registrar números
        </Botao>
      </div>
    </form>
  );
}

/** Resolução de divergência (gerência, docs/30 § 7). Quem pode resolver o servidor decide. */
export function ResolverDivergencia({ divergencia }: { divergencia: Divergencia }) {
  const { api } = useDependencias();
  const [aberto, setAberto] = useState(false);
  const [texto, setTexto] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((r: string, chave: string) => api.divergencias.resolver(divergencia.id ?? divergencia.codigo, r, { idempotencyKey: chave }), {
    aoConcluir: () => {
      setAberto(false);
    },
  });
  if (op.resultado !== null) {
    return <span className="t13 b" style={{ color: 'var(--green)' }}>Resolvida</span>;
  }
  return (
    <>
      <Botao
        pequeno
        onPress={() => {
          setAberto(true);
        }}
      >
        {`Resolver ${divergencia.codigo}`}
      </Botao>
      <DialogoDeFormulario
        aberto={aberto}
        titulo={`Resolver a divergência ${divergencia.codigo}?`}
        rotuloConfirmar="Registrar resolução"
        ocupado={op.enviando}
        aoFechar={() => {
          setAberto(false);
        }}
        aoConfirmar={() => {
          setTentou(true);
          if (texto.trim() !== '') op.enviar(texto.trim());
        }}
      >
        {divergencia.descricao !== undefined && (
          <p className="t14" style={{ margin: 0 }}>
            {divergencia.descricao}
          </p>
        )}
        <AreaDeTexto
          rotulo="Resolução"
          value={texto}
          onChange={setTexto}
          linhas={3}
          ajuda="O que foi conferido e decidido. Nenhum cadastro é unido ou apagado por esta tela."
          erro={tentou && texto.trim() === '' ? 'Escreva a resolução.' : null}
        />
        {op.erro !== null && <ErroDoAto erro={op.erro} ato="resolver a divergência" />}
      </DialogoDeFormulario>
    </>
  );
}
