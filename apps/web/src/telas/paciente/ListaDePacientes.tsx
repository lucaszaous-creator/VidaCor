import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { LinhaDaListaDePacientes, RecorteDaLista, SituacaoNaLista } from '@vidacor/api-client';
import { BotaoDeIcone, Chip, formatarData, formatarDataHora, ListaDeDados, useMenuDeContexto, Vazio, type CorDoChip } from '@vidacor/design-system';
import { useEffect, useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { MenuDoServidor } from '../../componentes/MenuDoServidor';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { ParesDuplicados } from './ParesDuplicados';

type Aba = RecorteDaLista | 'duplicados';

export const SITUACAO_NA_LISTA: Record<SituacaoNaLista, { texto: string; cor: CorDoChip }> = {
  na_clinica: { texto: 'Na clínica', cor: 'verdeAgua' },
  pendencia: { texto: 'Reserva sem guia', cor: 'ambar' },
  cadastro_incompleto: { texto: 'Cadastro incompleto', cor: 'rosa' },
  possivel_duplicado: { texto: 'Possível duplicado', cor: 'ambar' },
  em_dia: { texto: 'Em dia', cor: 'cinza' },
};

const FALTA: Record<string, string> = { cpf: 'CPF', celular: 'celular', nome_mae: 'nome da mãe', alergias: 'alergias' };

/** Idade em anos completos no dia de hoje da clínica (só exibição; nada é decidido com ela). */
export function idade(nascimento: string, hoje: Date = new Date()): string {
  const [a, m, d] = nascimento.split('-').map(Number);
  if (a === undefined || m === undefined || d === undefined) return '';
  let anos = hoje.getFullYear() - a;
  if (hoje.getMonth() + 1 < m || (hoje.getMonth() + 1 === m && hoje.getDate() < d)) anos -= 1;
  return `${String(anos)} anos`;
}

function proximoTexto(l: LinhaDaListaDePacientes): string {
  return l.proximo_exame === null || l.proximo_exame === undefined ? 'Sem agendamento' : `${l.proximo_exame.exame}, ${formatarDataHora(l.proximo_exame.inicio, FUSO_DA_CLINICA)}`;
}

function Linha({ l, atual, aoEscolher }: { l: LinhaDaListaDePacientes; atual: boolean; aoEscolher: () => void }) {
  const menu = useMenuDeContexto();
  const s = SITUACAO_NA_LISTA[l.situacao];
  const cabecalho = { titulo: l.nome, subtitulo: `${formatarData(l.nascimento)}, prontuário ${l.codigo}` };
  return (
    <tr className={atual ? 'sel' : undefined} aria-selected={atual}>
      <td>
        <button
          type="button"
          style={{ textAlign: 'left', border: 0, background: 'transparent', padding: 0, font: 'inherit', color: 'inherit', cursor: 'pointer', width: '100%' }}
          aria-label={`${l.nome}, nascimento ${formatarData(l.nascimento)}`}
          onClick={aoEscolher}
          {...menu.propsDoAlvo}
        >
          <b style={{ display: 'block' }}>{l.nome}</b>
          <span className="t13 mute num">
            {formatarData(l.nascimento)}, {idade(l.nascimento)}
          </span>
        </button>
        <MenuDoServidor estado={menu} objeto="paciente" id={l.id} cabecalho={cabecalho} />
      </td>
      <td className="t14">{l.convenio}</td>
      <td className="t14">{proximoTexto(l)}</td>
      <td>
        <Chip cor={s.cor} texto={s.texto} />
      </td>
      <td style={{ width: 44 }}>
        <BotaoDeIcone
          icone="maisOpcoes"
          estilo="mais"
          rotulo={`Mais opções de ${l.nome}`}
          onPress={(e) => {
            menu.abrirEm(e.target as HTMLElement);
          }}
        />
      </td>
    </tr>
  );
}

function Resumo({ l, podeAgendar }: { l: LinhaDaListaDePacientes | null; podeAgendar: boolean }) {
  if (l === null) return <Vazio titulo="Nenhum paciente selecionado." detalhe="Escolha um paciente na lista para ver o resumo." />;
  return (
    <div className="col" style={{ gap: 12 }}>
      <div>
        <h2 className="t15 b" style={{ margin: 0 }}>
          {l.nome}
        </h2>
        <span className="t14 mute num">
          {formatarData(l.nascimento)}, {idade(l.nascimento)}
        </span>
      </div>
      {l.alergias.length > 0 && <Chip cor="rosa" texto={`Alergia a ${l.alergias.join(', ')}`} />}
      <ListaDeDados
        larguraDoRotulo={110}
        itens={[
          { rotulo: 'Prontuário', valor: <span className="num">{l.codigo}</span> },
          { rotulo: 'Convênio', valor: l.convenio },
          { rotulo: 'CPF', valor: <span className="num">{l.cpf_mascarado ?? 'Não informado'}</span> },
          { rotulo: 'Celular', valor: <span className="num">{l.celular ?? 'Não informado'}</span> },
          { rotulo: 'Último exame', valor: l.ultimo_exame === null || l.ultimo_exame === undefined ? 'Nenhum' : `${l.ultimo_exame.exame}, ${formatarData(l.ultimo_exame.inicio.slice(0, 10))}` },
          { rotulo: 'Próximo', valor: proximoTexto(l) },
        ]}
      />
      {l.falta_no_cadastro.length > 0 && (
        <p className="t13" style={{ margin: 0 }}>
          Falta no cadastro: {l.falta_no_cadastro.map((f) => FALTA[f] ?? f).join(', ')}.
        </p>
      )}
      <div className="row wrap" style={{ gap: 8 }}>
        <Link to="/pacientes" search={{ id: l.id }} className="btn btn-primary">
          Abrir ficha
        </Link>
        {podeAgendar && (
          <Link to="/agendamentos/novo" className="btn">
            Agendar
          </Link>
        )}
      </div>
    </div>
  );
}

/** Lista de pacientes (Pacientes-Lista.dc.html): busca, recortes e páginas vêm do núcleo. */
export function ListaDePacientes() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const perfis = sessao.data?.usuario.perfis ?? [];
  const recepcao = perfis.includes('recepcao') || perfis.includes('gerente');
  const [texto, setTexto] = useState('');
  const [q, setQ] = useState('');
  const [aba, setAba] = useState<Aba>('todos');
  const [pagina, setPagina] = useState(1);
  const [escolhido, setEscolhido] = useState<string | null>(null);
  // Espera a pessoa parar de digitar antes de pedir ao servidor (sem pedido a cada letra).
  useEffect(() => {
    const t = setTimeout(() => {
      const limpo = texto.trim();
      setQ(limpo.length >= 3 || limpo === '' ? limpo : '');
      setPagina(1);
    }, 300);
    return () => {
      clearTimeout(t);
    };
  }, [texto]);
  const recorte: RecorteDaLista = aba === 'duplicados' ? 'todos' : aba;
  const lista = useQuery({
    queryKey: ['pacientes-lista', q, recorte, pagina],
    queryFn: () => api.pacientes.lista({ q, recorte, pagina }),
    placeholderData: keepPreviousData,
  });
  const d = lista.data;
  const atual = d?.itens.find((i) => i.id === escolhido) ?? d?.itens[0] ?? null;
  const abas: { id: Aba; rotulo: string; n: number | undefined; so?: boolean }[] = [
    { id: 'todos', rotulo: 'Todos', n: d?.recortes.todos },
    { id: 'hoje', rotulo: 'Na clínica hoje', n: d?.recortes.hoje },
    { id: 'pendencia', rotulo: 'Com pendência', n: d?.recortes.pendencia },
    { id: 'incompleto', rotulo: 'Cadastro incompleto', n: d?.recortes.incompleto },
    { id: 'duplicados', rotulo: 'Possíveis duplicados', n: d?.recortes.duplicados, so: true },
  ];
  return (
    <>
      <CabecalhoDaTela titulo="Pacientes" complemento={d === undefined ? undefined : <span className="t14 mute num">{d.cadastrados.toLocaleString('pt-BR')} cadastrados</span>}>
        <label className="row" style={{ gap: 8, flex: '1 1 320px', maxWidth: 520 }}>
          <input
            type="search"
            className="in"
            style={{ width: '100%' }}
            value={texto}
            placeholder="Nome, nascimento, CPF, prontuário ou celular"
            aria-label="Buscar paciente"
            onChange={(e) => {
              setTexto(e.target.value);
            }}
          />
        </label>
        {recepcao && (
          <Link to="/pacientes/novo" className="btn btn-primary">
            Novo paciente
          </Link>
        )}
      </CabecalhoDaTela>
      <nav className="tabs" aria-label="Recortes">
        {abas
          .filter((a) => a.so !== true || recepcao)
          .map((a) => (
            <button
              key={a.id}
              type="button"
              className={aba === a.id ? 'on' : undefined}
              aria-current={aba === a.id ? 'true' : undefined}
              onClick={() => {
                setAba(a.id);
                setPagina(1);
                setEscolhido(null);
              }}
            >
              {a.rotulo} {a.n === undefined ? '' : <span className="num mute">{a.n.toLocaleString('pt-BR')}</span>}
            </button>
          ))}
      </nav>
      {texto.trim().length > 0 && texto.trim().length < 3 && <p className="t13 mute" style={{ margin: 0 }}>Digite pelo menos 3 letras, o nascimento, o CPF ou o celular.</p>}
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 2.2fr) minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Lista de pacientes" style={{ padding: 0 }}>
          {aba === 'duplicados' ? (
            <div className="scroll" style={{ padding: 16 }}>
              <ParesDuplicados />
            </div>
          ) : (
            <>
              <div className="scroll">
                <EstadoDaConsulta oQue="a lista de pacientes" carregando={lista.isPending} erro={lista.error} aoTentarDeNovo={() => void lista.refetch()} />
                {d?.itens.length === 0 ? (
                  <div style={{ padding: 16 }}>
                    <Vazio titulo="Nenhum paciente encontrado." detalhe="Confira a grafia ou busque pela data de nascimento. Se for a primeira vez na clínica, cadastre." />
                  </div>
                ) : (
                  <table className="tb sticky">
                    <thead>
                      <tr>
                        <th>Paciente</th>
                        <th>Convênio</th>
                        <th>Próximo exame</th>
                        <th>Situação</th>
                        <th aria-label="Ações" />
                      </tr>
                    </thead>
                    <tbody>
                      {d?.itens.map((l) => (
                        <Linha
                          key={l.id}
                          l={l}
                          atual={atual?.id === l.id}
                          aoEscolher={() => {
                            setEscolhido(l.id);
                          }}
                        />
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
              {d !== undefined && d.total > 0 && (
                <footer className="row" style={{ gap: 8, padding: '10px 16px', borderTop: '1px solid var(--line)' }}>
                  <span className="t13 mute num">
                    {((d.pagina - 1) * d.por_pagina + 1).toLocaleString('pt-BR')} a {Math.min(d.pagina * d.por_pagina, d.total).toLocaleString('pt-BR')} de {d.total.toLocaleString('pt-BR')}
                  </span>
                  <div className="grow" />
                  <BotaoDeIcone
                    icone="anterior"
                    estilo="redondo"
                    rotulo="Página anterior"
                    isDisabled={d.pagina <= 1}
                    onPress={() => {
                      setPagina((p) => Math.max(1, p - 1));
                    }}
                  />
                  <span className="t13 num">
                    Página {d.pagina.toLocaleString('pt-BR')} de {d.paginas.toLocaleString('pt-BR')}
                  </span>
                  <BotaoDeIcone
                    icone="proximo"
                    estilo="redondo"
                    rotulo="Próxima página"
                    isDisabled={d.pagina >= d.paginas}
                    onPress={() => {
                      setPagina((p) => p + 1);
                    }}
                  />
                </footer>
              )}
            </>
          )}
        </section>
        <aside className="card pane" aria-label="Resumo do paciente">
          <div className="scroll">
            {aba === 'duplicados' ? (
              <p className="t14 mute" style={{ margin: 0 }}>
                Comparação por nome, nascimento, mãe e celular. Nada é juntado sem a conferência de outra pessoa; até lá os cadastros seguem separados.
              </p>
            ) : (
              <Resumo l={atual} podeAgendar={recepcao} />
            )}
          </div>
        </aside>
      </div>
    </>
  );
}
