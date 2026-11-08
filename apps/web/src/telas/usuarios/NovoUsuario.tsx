import { useQueryClient } from '@tanstack/react-query';
import type { NovoUsuario as Pedido } from '@vidacor/api-client';
import { Aviso, Botao, CaixaDeMarcar, Campo } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar } from '../../reautenticacao';
import { NOMES_DOS_PERFIS } from './TelaUsuarios';

/** Novo usuário: o login é o mesmo do provedor de identidade; perfil clínico espera a segunda pessoa. */
export function NovoUsuario({ aoCriar }: { aoCriar: (id: string) => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const reautenticar = useReautenticar();
  const [login, setLogin] = useState('');
  const [nome, setNome] = useState('');
  const [motivo, setMotivo] = useState('');
  const [perfis, setPerfis] = useState<string[]>([]);
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((p: Pedido, chave: string) => api.usuarios.criar(p, { idempotencyKey: chave }), {
    aoConcluir: async (u) => {
      await cliente.invalidateQueries({ queryKey: ['usuarios'] });
      aoCriar(u.id);
    },
  });
  const ok = login.trim().length >= 3 && nome.trim().length >= 3 && motivo.trim().length >= 5 && perfis.length > 0;
  const enviar = () => {
    setTentou(true);
    if (ok) op.enviar({ login: login.trim(), nome: nome.trim(), perfis, motivo: motivo.trim() });
  };
  return (
    <form className="col" style={{ gap: 10 }} noValidate onSubmit={(ev) => { ev.preventDefault(); enviar(); }}>
      <h2 className="t15 b" style={{ margin: 0 }}>
        Novo usuário
      </h2>
      <Campo rotulo="Login (o mesmo do acesso da clínica)" value={login} onChange={setLogin} autoComplete="off" erro={tentou && login.trim().length < 3 ? 'Digite o login.' : null} />
      <Campo rotulo="Nome completo" value={nome} onChange={setNome} erro={tentou && nome.trim().length < 3 ? 'Digite o nome.' : null} />
      <fieldset className="col" style={{ gap: 4, border: 0, padding: 0, margin: 0 }}>
        <legend className="t13 b">Perfis</legend>
        <div className="grid g2" style={{ gap: 4 }}>
          {Object.entries(NOMES_DOS_PERFIS).map(([k, rotulo]) => (
            <CaixaDeMarcar key={k} rotulo={rotulo} marcado={perfis.includes(k)} aoMudar={(m) => { setPerfis((ps) => (m ? [...ps, k] : ps.filter((p) => p !== k))); }} />
          ))}
        </div>
        {tentou && perfis.length === 0 && <span className="campo-erro">Escolha ao menos um perfil.</span>}
      </fieldset>
      {perfis.some((p) => ['medico', 'tecnico', 'enfermagem', 'auditor'].includes(p)) && (
        <Aviso nivel="informativo" titulo="Perfil com dado clínico.">
          O usuário nasce inativo e só entra depois da conferência de outra pessoa da gerência.
        </Aviso>
      )}
      <Campo rotulo="Motivo" value={motivo} onChange={setMotivo} erro={tentou && motivo.trim().length < 5 ? 'Diga o motivo (ex.: contratação).' : null} />
      {op.erro !== null && (
        <ErroDoAto erro={op.erro} ato="criar o usuário" aoReautenticar={() => { reautenticar('criar-usuario', op.chave(), {}, enviar); }} />
      )}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Criar usuário
        </Botao>
      </div>
    </form>
  );
}
