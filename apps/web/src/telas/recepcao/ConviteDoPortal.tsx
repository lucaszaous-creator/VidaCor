import type { ConviteEmitido } from '@vidacor/api-client';
import { Aviso, Botao, formatarDataHora } from '@vidacor/design-system';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

/**
 * Convite do portal pela recepção (docs/04 Cadastro, SEC-AUT-6, DEC-2026-11-08-60). O link vai por
 * e-mail para o endereço do cadastro e não aparece aqui; é de uso único, vale 72 h e pede a data de
 * nascimento ao abrir. Convite novo invalida o anterior (serve também para quem esqueceu a senha).
 */
export function ConviteDoPortal({ pacienteId, nome }: { pacienteId: string; nome: string }) {
  const { api } = useDependencias();
  const op = useOperacao((_: undefined, chave: string) => api.pacientes.convidarParaPortal(pacienteId, { idempotencyKey: chave }));
  const r: ConviteEmitido | null = op.resultado;

  if (r !== null) {
    return (
      <Aviso
        nivel="feito"
        titulo={`Convite do portal enviado para ${nome}.`}
        acoes={
          <Botao pequeno onPress={op.reiniciar}>
            Enviar outro
          </Botao>
        }
      >
        E-mail para <span className="num">{r.enviado_para}</span>, com link de uso único válido até {formatarDataHora(r.expira_em, FUSO_DA_CLINICA)}.
        Se o paciente não receber, confira o e-mail do cadastro e envie outro: o anterior deixa de valer.
      </Aviso>
    );
  }
  return (
    <div className="col" style={{ gap: 8 }}>
      {op.erro !== null && (
        <ErroDoAto
          erro={op.erro}
          ato="convidar para o portal"
          aoTentarDeNovo={() => {
            op.enviar(undefined);
          }}
        />
      )}
      <div className="row">
        <Botao
          pequeno
          ocupado={op.enviando}
          onPress={() => {
            op.enviar(undefined);
          }}
        >
          Convidar para o portal por e-mail
        </Botao>
      </div>
    </div>
  );
}
