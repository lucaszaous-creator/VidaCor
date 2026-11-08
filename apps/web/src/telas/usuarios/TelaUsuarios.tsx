import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { MudancaDePerfil, UsuarioAdministrado } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Chip, Escolha, formatarDataHora, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { NovoUsuario } from './NovoUsuario';

export const NOMES_DOS_PERFIS: Record<string, string> = {
  recepcao: 'Recepção', caixa: 'Caixa', tecnico: 'Técnico', enfermagem: 'Enfermagem', medico: 'Médico', faturista: 'Faturista',
  financeiro: 'Financeiro', gerente: 'Gerência', estoque: 'Estoque', auditor: 'Auditor', ti: 'TI',
};
const CHAVE = ['usuarios'];

/**
 * Usuários e perfis (Usuarios-Perfis.dc.html). Perfil clínico e reativação esperam a segunda pessoa;
 * desativar derruba a sessão na hora. O servidor confere separação de funções e pede a senha de novo.
 */
export function TelaUsuarios() {
  const { api } = useDependencias();
  const lista = useQuery({ queryKey: CHAVE, queryFn: () => api.usuarios.listar() });
  const [selecionado, setSelecionado] = useState<string | null>(null);
  const [novo, setNovo] = useState(false);
  const u = lista.data?.find((x) => x.id === selecionado) ?? null;
  return (
    <>
      <CabecalhoDaTela titulo="Usuários e perfis" complemento={<span className="t14 mute">Mudar permissão pede a sua senha de novo</span>}>
        <Botao pequeno variante="principal" onPress={() => { setNovo(true); setSelecionado(null); }}>
          Novo usuário
        </Botao>
      </CabecalhoDaTela>
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Usuários" style={{ padding: 0 }}>
          <div className="scroll">
            <EstadoDaConsulta
              oQue="os usuários"
              carregando={lista.isPending}
              erro={lista.error}
              aoTentarDeNovo={() => void lista.refetch()}
              semPermissao={{ titulo: 'Você não administra usuários', detalhe: 'Contas e perfis são da TI e da gerência.' }}
            />
            {lista.data?.map((x) => (
              <button key={x.id} type="button" className="lp" aria-current={x.id === selecionado} onClick={() => { setSelecionado(x.id); setNovo(false); }}>
                <span style={{ flex: 1, minWidth: 0 }}>
                  <span className="t clamp1" style={{ display: 'block' }}>
                    {x.nome}
                  </span>
                  <span className="s" style={{ display: 'block' }}>
                    {x.login}, {x.perfis.map((p) => NOMES_DOS_PERFIS[p] ?? p).join(', ')}
                  </span>
                </span>
                {x.pendente !== null && x.pendente !== undefined ? <Chip cor="ambar" texto="Aguarda conferência" /> : x.ativo ? <Chip cor="verde" texto="Ativo" /> : <Chip cor="cinza" texto="Inativo" />}
              </button>
            ))}
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe do usuário">
          <div className="scroll col" style={{ gap: 14 }}>
            {novo ? <NovoUsuario aoCriar={(id) => { setNovo(false); setSelecionado(id); }} /> : u === null ? <Vazio titulo="Escolha um usuário." /> : <DetalheDoUsuario key={u.id} u={u} />}
          </div>
        </section>
      </div>
    </>
  );
}

function DetalheDoUsuario({ u }: { u: UsuarioAdministrado }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const sessao = useQuery(consultaSessao(api));
  const reautenticar = useReautenticar();
  const [motivo, setMotivo] = useState('');
  const [perfil, setPerfil] = useState('');
  const [tentou, setTentou] = useState(false);
  const atualizar = () => void cliente.invalidateQueries({ queryKey: CHAVE });
  const mudar = useOperacao((m: MudancaDePerfil, chave: string) => api.usuarios.mudar(u.id, m, { idempotencyKey: chave }), { aoConcluir: atualizar });
  const decidir = useOperacao((aprovar: boolean, chave: string) => api.usuarios.decidir(u.pendente?.id ?? '', { aprovar, motivo: motivo.trim() }, { idempotencyKey: chave }), { aoConcluir: atualizar });
  const motivoOk = motivo.trim().length >= 5;
  const pedir = (m: Omit<MudancaDePerfil, 'motivo'>) => {
    setTentou(true);
    if (motivoOk) mudar.enviar({ ...m, motivo: motivo.trim() });
  };
  const ehGerente = sessao.data?.usuario.perfis.includes('gerente') ?? false;
  const erro = mudar.erro ?? decidir.erro;
  return (
    <>
      <h2 style={{ fontSize: 20, fontWeight: 650, margin: 0 }}>{u.nome}</h2>
      <div className="t14 mute">
        {u.login}, {u.codigo}. Último acesso: {typeof u.ultimo_acesso === 'string' ? formatarDataHora(u.ultimo_acesso, FUSO_DA_CLINICA) : 'nunca'}
      </div>
      {u.pendente !== null && u.pendente !== undefined && (
        <Aviso nivel="atencao" titulo={`Aguarda a conferência de outra pessoa: ${u.pendente.tipo}${typeof u.pendente.perfil === 'string' ? ` ${NOMES_DOS_PERFIS[u.pendente.perfil] ?? u.pendente.perfil}` : ''}.`}>
          Pedido por {u.pendente.pedido_por}. Motivo: {u.pendente.motivo}.
        </Aviso>
      )}
      <h3 className="t15 b" style={{ margin: 0 }}>
        Perfis
      </h3>
      {u.perfis.map((p) => (
        <div key={p} className="li" style={{ padding: '6px 0' }}>
          <span style={{ flex: 1 }}>{NOMES_DOS_PERFIS[p] ?? p}</span>
          {u.perfis.length > 1 && (
            <Botao pequeno onPress={() => { pedir({ tipo: 'retirar', perfil: p }); }}>
              Retirar
            </Botao>
          )}
        </div>
      ))}
      <Campo rotulo="Motivo da mudança" value={motivo} onChange={setMotivo} erro={tentou && !motivoOk ? 'Diga o motivo (mínimo 5 letras).' : null} />
      <div className="row" style={{ gap: 8, alignItems: 'flex-end' }}>
        <div style={{ width: 220 }}>
          <Escolha
            rotulo="Adicionar perfil"
            valor={perfil}
            aoMudar={setPerfil}
            vazio="Escolha"
            opcoes={Object.entries(NOMES_DOS_PERFIS)
              .filter(([k]) => !u.perfis.includes(k))
              .map(([valor, rotulo]) => ({ valor, rotulo }))}
          />
        </div>
        <Botao isDisabled={perfil === ''} onPress={() => { pedir({ tipo: 'conceder', perfil }); }}>
          Adicionar
        </Botao>
        {u.ativo ? (
          <Botao onPress={() => { pedir({ tipo: 'desativar' }); }}>Desativar</Botao>
        ) : (
          <Botao onPress={() => { pedir({ tipo: 'reativar' }); }}>Reativar (pede segunda pessoa)</Botao>
        )}
      </div>
      {u.pendente !== null && u.pendente !== undefined && ehGerente && (
        <div className="row" style={{ gap: 8 }}>
          <Botao variante="principal" onPress={() => { setTentou(true); if (motivoOk) decidir.enviar(true); }}>
            Conferir e aplicar
          </Botao>
          <Botao onPress={() => { setTentou(true); if (motivoOk) decidir.enviar(false); }}>Recusar</Botao>
        </div>
      )}
      {erro !== null && (
        <ErroDoAto
          erro={erro}
          ato="mudar a permissão"
          aoReautenticar={() => {
            const op = mudar.erro !== null ? mudar : decidir;
            reautenticar('mudar-permissao', op.chave(), {}, () => { setTentou(false); });
          }}
        />
      )}
      {(mudar.resultado !== null || decidir.resultado !== null) && erro === null && (
        <span className="t13" role="status">
          Mudança registrada no lastro.
        </span>
      )}
    </>
  );
}
