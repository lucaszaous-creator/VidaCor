import { getRouteApi } from '@tanstack/react-router';
import { Aviso, Botao, Campo, DialogoDeFormulario } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar, useRetomada } from '../../reautenticacao';

const rota = getRouteApi('/app/caixa/estornos');
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * Aprovação de estorno (FC-CXA-09), com reautenticação. Quem pediu não aprova: o servidor responde
 * `403 separacao-de-funcoes` e a tela mostra a mensagem dele.
 */
export function TelaEstornos() {
  const { api } = useDependencias();
  const busca = rota.useSearch();
  const retomada = useRetomada();
  const reautenticar = useReautenticar();
  const [codigo, setCodigo] = useState(busca.estorno ?? '');
  const [tentou, setTentou] = useState(false);
  const [confirmando, setConfirmando] = useState(retomada?.retomar === 'aprovar-estorno');
  const op = useOperacao((id: string, chave: string) => api.estornos.aprovar(id, { idempotencyKey: chave }), {
    chaveInicial: retomada?.retomar === 'aprovar-estorno' ? retomada.chave : undefined,
    aoConcluir: () => {
      setConfirmando(false);
    },
    aoFalhar: () => {
      setConfirmando(false);
    },
  });
  const valido = UUID.test(codigo.trim());

  return (
    <>
      <CabecalhoDaTela titulo="Aprovar estorno" />
      <section className="card pane fill" aria-label="Aprovação de estorno">
        <div className="scroll col" style={{ gap: 14, maxWidth: 640 }}>
          {op.resultado !== null ? (
            <Aviso nivel="feito" titulo="Estorno aprovado.">
              Pedido <b className="num">{op.resultado.id}</b>, motivo: {op.resultado.motivo}. O estorno ao paciente segue pelo provedor; o lastro grava quem
              pediu e quem aprovou.
            </Aviso>
          ) : (
            <form
              className="col"
              style={{ gap: 12 }}
              noValidate
              onSubmit={(ev) => {
                ev.preventDefault();
                setTentou(true);
                if (valido) setConfirmando(true);
              }}
            >
              <Campo
                rotulo="Código do pedido de estorno"
                value={codigo}
                onChange={setCodigo}
                autoComplete="off"
                ajuda="Vem no endereço enviado por quem pediu o estorno."
                erro={tentou && !valido ? 'Cole o código do pedido como foi enviado.' : null}
              />
              {op.erro !== null && (
                <ErroDoAto
                  erro={op.erro}
                  ato="aprovar o estorno"
                  aoReautenticar={() => {
                    reautenticar('aprovar-estorno', op.chave(), { estorno: codigo.trim() });
                  }}
                  aoTentarDeNovo={() => {
                    op.enviar(codigo.trim());
                  }}
                />
              )}
              <div className="row">
                <Botao type="submit" variante="principal">
                  Revisar e aprovar
                </Botao>
              </div>
            </form>
          )}
        </div>
      </section>
      <DialogoDeFormulario
        aberto={confirmando}
        irreversivel
        titulo="Aprovar este estorno?"
        rotuloConfirmar="Aprovar estorno"
        ocupado={op.enviando}
        aoFechar={() => {
          setConfirmando(false);
        }}
        aoConfirmar={() => {
          op.enviar(codigo.trim());
        }}
      >
        <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
          Pedido <b className="num">{codigo.trim()}</b>. O valor volta ao paciente e não há como desfazer. Quem pediu o estorno não pode aprovar.
        </p>
      </DialogoDeFormulario>
    </>
  );
}
