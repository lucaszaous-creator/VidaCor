import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { GrandezaDeDose, NovoNrd, NrdDoExame } from '@vidacor/api-client';
import { Botao, Campo, Escolha, Gaveta, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { NOME_DA_GRANDEZA, numero, UNIDADE } from './formatos';

function lerNumero(texto: string): number | null {
  const n = Number(texto.trim().replace(/\./g, '').replace(',', '.'));
  return texto.trim() === '' || !Number.isFinite(n) ? null : n;
}

/** Publica versão nova do NRD de um exame (gerência). A anterior fica guardada. */
function PublicarNrd({ exame, aoFechar }: { exame: NrdDoExame; aoFechar: () => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const tc = exame.modalidade === 'CT';
  const [grandeza, setGrandeza] = useState<string>(exame.grandeza ?? (tc ? 'ctdivol' : 'dap'));
  const [valor, setValor] = useState(typeof exame.valor === 'number' ? String(exame.valor).replace('.', ',') : '');
  const [fator, setFator] = useState(typeof exame.fator_dose_efetiva === 'number' ? String(exame.fator_dose_efetiva).replace('.', ',') : '');
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovoNrd, chave: string) => api.doses.publicarNrd(exame.tuss, n, { idempotencyKey: chave }), {
    aoConcluir: async () => {
      await cliente.invalidateQueries({ queryKey: ['nrd'] });
      aoFechar();
    },
  });
  const v = lerNumero(valor);
  const k = lerNumero(fator);
  const erroValor = v === null || v <= 0 || v > 100_000 ? 'Informe um valor maior que zero.' : null;
  const erroFator = fator.trim() !== '' && (k === null || k <= 0 || k > 10) ? 'O fator vai de 0 a 10.' : null;
  const precisaMotivo = typeof exame.numero === 'number';
  const erroMotivo = precisaMotivo && motivo.trim().length < 5 ? 'Diga por que o NRD mudou (5 letras ou mais).' : null;
  const opcoes = (tc ? (['ctdivol', 'dlp'] as GrandezaDeDose[]) : (['dap'] as GrandezaDeDose[])).map((g) => ({ valor: g, rotulo: `${NOME_DA_GRANDEZA[g]} (${UNIDADE[g]})` }));
  return (
    <Gaveta titulo={typeof exame.numero === 'number' ? `Nova versão do NRD (hoje v${String(exame.numero)})` : 'Cadastrar o NRD'} subTitulo={exame.exame} rotulo="NRD do exame" aoFechar={aoFechar}>
      <form
        className="col"
        style={{ gap: 10 }}
        aria-label="NRD do exame"
        noValidate
        onSubmit={(ev) => {
          ev.preventDefault();
          setTentou(true);
          if (erroValor === null && erroFator === null && erroMotivo === null && v !== null)
            op.enviar({
              versao_vista: exame.numero ?? null,
              grandeza: grandeza as GrandezaDeDose,
              valor: v,
              fator_dose_efetiva: k,
              motivo: motivo.trim() === '' ? null : motivo.trim(),
            });
        }}
      >
        <Escolha rotulo="Grandeza" valor={grandeza} aoMudar={setGrandeza} opcoes={opcoes} />
        <Campo rotulo={`Valor do NRD (${UNIDADE[grandeza as GrandezaDeDose]})`} value={valor} onChange={setValor} erro={tentou ? erroValor : null} inputMode="decimal" />
        <Campo
          rotulo="Fator para dose efetiva (opcional)"
          ajuda={tc ? 'mSv por mGy·cm de DLP, da região do exame.' : 'mSv por Gy·cm² de DAP.'}
          value={fator}
          onChange={setFator}
          erro={tentou ? erroFator : null}
          inputMode="decimal"
        />
        {precisaMotivo && <Campo rotulo="Motivo da mudança" value={motivo} onChange={setMotivo} erro={tentou ? erroMotivo : null} />}
        <p className="t13 mute" style={{ margin: 0 }}>
          Os valores são da clínica e da física médica. Doses já lidas continuam comparadas à versão que valia na leitura.
        </p>
        {op.erro !== null && <ErroDoAto erro={op.erro} ato="publicar o NRD" />}
        <div className="row" style={{ gap: 8 }}>
          <Botao type="submit" variante="principal" ocupado={op.enviando}>
            Publicar NRD
          </Botao>
          <Botao onPress={aoFechar}>Voltar</Botao>
        </div>
      </form>
    </Gaveta>
  );
}

/** Agregado por protocolo (sem paciente) e NRD vigente de cada exame com radiação. */
export function ProtocolosENrd() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const protocolos = useQuery({ queryKey: ['dose-protocolos'], queryFn: () => api.doses.protocolos() });
  const nrd = useQuery({ queryKey: ['nrd'], queryFn: () => api.doses.nrd() });
  const [editar, setEditar] = useState<NrdDoExame | null>(null);
  const gerencia = sessao.data?.usuario.perfis.includes('gerente') ?? false;
  if (protocolos.data === undefined || nrd.data === undefined) {
    return (
      <EstadoDaConsulta
        oQue="os protocolos de dose"
        carregando={protocolos.isPending || nrd.isPending}
        erro={protocolos.error ?? nrd.error}
        aoTentarDeNovo={() => {
          void protocolos.refetch();
          void nrd.refetch();
        }}
      />
    );
  }
  return (
    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: 16, height: 'calc(100vh - 260px)', minHeight: 300 }}>
      <section aria-label="Dose por protocolo" className="col" style={{ minHeight: 0, overflowY: 'auto', gap: 8 }}>
        <h2 className="t15 b" style={{ margin: 0 }}>
          Mediana dos últimos 7 dias
        </h2>
        {protocolos.data.length === 0 ? (
          <Vazio titulo="Nenhum exame com dose no período." />
        ) : (
          <ul className="col" style={{ gap: 10, listStyle: 'none', margin: 0, padding: 0 }}>
            {protocolos.data.map((p) => (
              <li key={`${p.exame_tuss}-${p.aparelho}`}>
                <div className="row t14" style={{ gap: 8 }}>
                  <b style={{ flex: 1, minWidth: 0 }}>{p.exame}</b>
                  <span className="num">{p.exames} exames</span>
                </div>
                <div className="t13 mute">
                  {p.aparelho}
                  {typeof p.mediana === 'number' && p.grandeza !== '' ? `, ${NOME_DA_GRANDEZA[p.grandeza as keyof typeof NOME_DA_GRANDEZA]} mediano ${numero(p.mediana)}` : ', sem NRD na leitura'}
                  {typeof p.percentual_do_nrd === 'number' ? `, ${String(p.percentual_do_nrd)}% do NRD` : ''}
                  {p.acima > 0 ? `, ${String(p.acima)} acima` : ''}
                </div>
                {typeof p.percentual_do_nrd === 'number' && (
                  <div
                    role="progressbar"
                    aria-label={`Mediana de ${p.exame} em relação ao NRD`}
                    aria-valuenow={p.percentual_do_nrd}
                    aria-valuemin={0}
                    aria-valuemax={100}
                    style={{ height: 6, borderRadius: 3, background: 'var(--line)', marginTop: 2 }}
                  >
                    <div style={{ width: `${String(Math.min(100, p.percentual_do_nrd))}%`, height: 6, borderRadius: 3, background: 'var(--navy, #0A1A4A)' }} />
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>
      <section aria-label="NRD da clínica" className="col" style={{ minHeight: 0, overflowY: 'auto', gap: 8 }}>
        <h2 className="t15 b" style={{ margin: 0 }}>
          NRD da clínica
        </h2>
        {nrd.data.length === 0 ? (
          <Vazio titulo="Nenhum exame de TC ou raio X ativo no catálogo." />
        ) : (
          <ul className="col" style={{ gap: 0, listStyle: 'none', margin: 0, padding: 0 }}>
            {nrd.data.map((n) => (
              <li key={n.tuss} className="row" style={{ padding: '8px 0', borderBottom: '1px solid var(--line)', gap: 8, alignItems: 'flex-start' }}>
                <span style={{ flex: 1, minWidth: 0 }}>
                  <b style={{ display: 'block' }}>{n.exame}</b>
                  <span className="t13 mute">
                    {typeof n.valor === 'number' && n.grandeza !== null && n.grandeza !== undefined
                      ? `${NOME_DA_GRANDEZA[n.grandeza]} ${numero(n.valor)} ${UNIDADE[n.grandeza]}, v${String(n.numero ?? 1)}`
                      : 'Sem NRD cadastrado'}
                  </span>
                </span>
                {gerencia && (
                  <Botao
                    pequeno
                    onPress={() => {
                      setEditar(n);
                    }}
                  >
                    {typeof n.numero === 'number' ? 'Nova versão' : 'Cadastrar'}
                  </Botao>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>
      {editar !== null && (
        <PublicarNrd
          exame={editar}
          aoFechar={() => {
            setEditar(null);
          }}
        />
      )}
    </div>
  );
}
