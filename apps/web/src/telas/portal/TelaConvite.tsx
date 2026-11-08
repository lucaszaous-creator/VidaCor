import type { CalendarDate } from '@internationalized/date';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getRouteApi, useNavigate } from '@tanstack/react-router';
import { Aviso, Botao, Campo, CampoDeData, Carregando } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { ErroDoPortal } from './TelaPortal';

const rota = getRouteApi('/portal/convite/$token');

/**
 * Aceite do convite do portal: o paciente confirma a data de nascimento (segundo identificador) e
 * cria login e senha. O servidor confere tudo, conta as tentativas e aceita o link uma vez só.
 */
export function TelaConvite() {
  const { api } = useDependencias();
  const { token } = rota.useParams();
  const navegar = useNavigate();
  const cliente = useQueryClient();
  const situacao = useQuery({ queryKey: ['convite', token], queryFn: () => api.portal.situacaoDoConvite(token), retry: false });
  const [nascimento, setNascimento] = useState<CalendarDate | null>(null);
  const [login, setLogin] = useState('');
  const [senha, setSenha] = useState('');
  const [repetida, setRepetida] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao(() => api.portal.aceitarConvite(token, { nascimento: nascimento?.toString() ?? '', login: login.trim(), senha }), {
    aoConcluir: async () => {
      await cliente.invalidateQueries();
      await navegar({ to: '/portal' });
    },
  });
  const erros = {
    nascimento: tentou && nascimento === null ? 'Digite a sua data de nascimento.' : null,
    login: tentou && login.trim().length < 4 ? 'Escolha um login com pelo menos 4 letras ou números.' : null,
    senha: tentou && senha.length < 10 ? 'A senha precisa ter pelo menos 10 caracteres.' : null,
    repetida: tentou && repetida !== senha ? 'As duas senhas não são iguais.' : null,
  };

  if (situacao.isPending) return <Carregando rotulo="Conferindo o convite" />;
  if (situacao.isError) return <ErroDoPortal erro={situacao.error} aoTentarDeNovo={() => void situacao.refetch()} />;
  if (situacao.data.situacao !== 'valido') {
    return (
      <Aviso nivel="atencao" titulo="Este convite não vale mais.">
        Ele venceu, já foi usado ou foi trocado por outro. Nada foi alterado. Peça um convite novo na recepção da VidaCor.
      </Aviso>
    );
  }
  return (
    <section className="card" aria-label="Primeiro acesso" style={{ maxWidth: 520, margin: '24px auto', width: '100%' }}>
      <h1 style={{ fontSize: 24, fontWeight: 650, margin: '0 0 6px' }}>Primeiro acesso</h1>
      <p className="t14 mute" style={{ margin: '0 0 16px', lineHeight: 1.5 }}>
        Confirme a sua data de nascimento e crie o login e a senha do portal.
      </p>
      <form
        className="col"
        style={{ gap: 12 }}
        noValidate
        onSubmit={(ev) => {
          ev.preventDefault();
          setTentou(true);
          if (Object.values(erros).every((e) => e === null) && nascimento !== null && login.trim().length >= 4 && senha.length >= 10 && repetida === senha) {
            op.enviar(undefined);
          }
        }}
      >
        <CampoDeData rotulo="Data de nascimento" valor={nascimento} aoMudar={setNascimento} erro={erros.nascimento} />
        <Campo rotulo="Login" value={login} onChange={setLogin} autoComplete="username" ajuda="Letras sem acento, números ou . _ @ + -" erro={erros.login} />
        <Campo rotulo="Senha" value={senha} onChange={setSenha} type="password" autoComplete="new-password" ajuda="Pelo menos 10 caracteres." erro={erros.senha} />
        <Campo rotulo="Repita a senha" value={repetida} onChange={setRepetida} type="password" autoComplete="new-password" erro={erros.repetida} />
        {op.erro !== null && <ErroDoAto erro={op.erro} ato="criar o acesso" />}
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Criar acesso e entrar
        </Botao>
      </form>
    </section>
  );
}
