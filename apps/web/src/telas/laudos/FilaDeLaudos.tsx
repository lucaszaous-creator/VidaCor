import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import type { CorDoChip } from '@vidacor/design-system';
import { Vazio } from '@vidacor/design-system';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { LinhaDoPaciente } from '../../componentes/LinhaDoPaciente';
import { consultaFilaDeLaudos } from '../../consultas';
import { useDependencias } from '../../contexto';

export const SITUACAO_DO_LAUDO: Record<string, { texto: string; cor: CorDoChip }> = {
  rascunho: { texto: 'Rascunho', cor: 'azul' },
  assinado: { texto: 'Assinado', cor: 'verde' },
  retificacao_em_rascunho: { texto: 'Retificação', cor: 'ambar' },
};

/** "Minha fila" (Laudo.dc.html e Fluxo-Laudos.dc.html): só laudos atribuídos a mim, com estudo conciliado. */
export function FilaDeLaudos({ atual }: { atual?: string }) {
  const { api } = useDependencias();
  const navegar = useNavigate();
  const fila = useQuery(consultaFilaDeLaudos(api));
  return (
    <section className="card pane" aria-label="Minha fila de laudos" style={{ padding: 0 }}>
      <div className="card-h" style={{ padding: '14px 16px 0', marginBottom: 8 }}>
        <h2>Minha fila</h2>
        <span className="t13 mute">{fila.data === undefined ? '' : String(fila.data.length)}</span>
      </div>
      <div className="scroll">
        <EstadoDaConsulta
          oQue="a fila de laudos"
          carregando={fila.isPending}
          erro={fila.error}
          aoTentarDeNovo={() => void fila.refetch()}
          semPermissao={{ titulo: 'Você não tem fila de laudos', detalhe: 'A fila aparece para médicos com laudos atribuídos.' }}
        />
        {fila.data?.length === 0 && (
          <div style={{ padding: 16 }}>
            <Vazio titulo="Nenhum laudo na sua fila" detalhe="Laudos novos aparecem quando o estudo é conciliado." />
          </div>
        )}
        {fila.data?.map((l) => (
          <LinhaDoPaciente
            key={l.id}
            nome={l.paciente.nome}
            nascimento={l.paciente.nascimento}
            detalhe={l.exame}
            situacao={SITUACAO_DO_LAUDO[l.situacao] ?? { texto: 'Situação não reconhecida', cor: 'cinza' }}
            marca={l.conversar_antes ? 'Conversar antes de liberar' : undefined}
            selecionada={l.id === atual}
            contextoDoMenu={l.exame}
            menuDe={{ objeto: 'laudo', id: l.id }}
            aoAbrir={() => {
              void navegar({ to: '/laudos/$laudoId', params: { laudoId: l.id } });
            }}
          />
        ))}
      </div>
    </section>
  );
}
