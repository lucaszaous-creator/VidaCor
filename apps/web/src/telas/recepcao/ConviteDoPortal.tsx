import type { ConviteEmitido } from '@vidacor/api-client';
import { Aviso, Botao, formatarDataHora } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

/**
 * Convite do portal pela recepção (docs/04 Cadastro, SEC-AUT-6). O link é de uso único, vale 72 h e
 * aparece só agora; o paciente confirma a data de nascimento ao abrir. Convite novo invalida o
 * anterior (serve também para quem esqueceu a senha).
 */
export function ConviteDoPortal({ pacienteId, nome }: { pacienteId: string; nome: string }) {
  const { api } = useDependencias();
  const [copiado, setCopiado] = useState(false);
  const op = useOperacao((_: undefined, chave: string) => api.pacientes.convidarParaPortal(pacienteId, { idempotencyKey: chave }));
  const r: ConviteEmitido | null = op.resultado;

  if (r !== null) {
    const link = typeof r.caminho === 'string' ? new URL(r.caminho, globalThis.location.origin).toString() : null;
    return (
      <Aviso
        nivel="feito"
        titulo={`Convite do portal para ${nome}.`}
        acoes={
          <>
            {link !== null && (
              <Botao
                pequeno
                variante="principal"
                onPress={() => {
                  if ('clipboard' in navigator) void navigator.clipboard.writeText(link).then(() => { setCopiado(true); });
                }}
              >
                {copiado ? 'Link copiado' : 'Copiar link'}
              </Botao>
            )}
            <Botao pequeno onPress={op.reiniciar}>
              Gerar outro
            </Botao>
          </>
        }
      >
        {link === null ? (
          'O link já foi mostrado e não fica guardado. Para enviar de novo, gere outro: o anterior deixa de valer.'
        ) : (
          <>
            Envie ao paciente: <span className="num" style={{ wordBreak: 'break-all' }}>{link}</span>. Vale até{' '}
            {formatarDataHora(r.expira_em, FUSO_DA_CLINICA)}, uma vez só; ao abrir, o paciente confirma a data de nascimento e cria a senha. O link
            não aparece de novo.
          </>
        )}
      </Aviso>
    );
  }
  return (
    <div className="col" style={{ gap: 8 }}>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="convidar para o portal" aoTentarDeNovo={() => { op.enviar(undefined); }} />}
      <div className="row">
        <Botao pequeno ocupado={op.enviando} onPress={() => { op.enviar(undefined); }}>
          Convidar para o portal
        </Botao>
      </div>
    </div>
  );
}
