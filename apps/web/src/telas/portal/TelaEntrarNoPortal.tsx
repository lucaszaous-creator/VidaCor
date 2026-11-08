import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { Botao, Campo } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

/**
 * Login do paciente (Login.dc.html, versão do portal). Mensagem única para login ou senha errados;
 * quem esqueceu a senha pede um convite novo na recepção (SEC-AUT-6). Nada fica no navegador além do
 * cookie de sessão HttpOnly.
 */
export function TelaEntrarNoPortal() {
  const { api } = useDependencias();
  const navegar = useNavigate();
  const cliente = useQueryClient();
  const [login, setLogin] = useState('');
  const [senha, setSenha] = useState('');
  const [ver, setVer] = useState(false);
  const op = useOperacao(() => api.portal.entrar({ login: login.trim(), senha }), {
    aoConcluir: async () => {
      await cliente.invalidateQueries();
      await navegar({ to: '/portal' });
    },
  });
  const pronto = login.trim() !== '' && senha !== '';
  return (
    <section className="card" aria-label="Entrar no portal" style={{ maxWidth: 440, margin: '24px auto', width: '100%' }}>
      <h1 style={{ fontSize: 24, fontWeight: 650, margin: '0 0 16px' }}>Entrar</h1>
      <form
        className="col"
        style={{ gap: 12 }}
        noValidate
        onSubmit={(ev) => {
          ev.preventDefault();
          if (pronto) op.enviar(undefined);
        }}
      >
        <Campo rotulo="Login" value={login} onChange={setLogin} autoComplete="username" />
        <Campo rotulo="Senha" value={senha} onChange={setSenha} type={ver ? 'text' : 'password'} autoComplete="current-password" />
        <div className="row">
          <Botao pequeno onPress={() => { setVer((v) => !v); }}>
            {ver ? 'Esconder a senha' : 'Mostrar a senha'}
          </Botao>
        </div>
        {op.erro !== null && <ErroDoAto erro={op.erro} ato="entrar" />}
        <Botao type="submit" variante="principal" ocupado={op.enviando} isDisabled={!pronto}>
          Entrar
        </Botao>
        <p className="t13 mute" style={{ margin: 0, lineHeight: 1.5 }}>
          Primeiro acesso ou esqueceu a senha? Peça um convite na recepção: o link chega por mensagem e vale 72 horas.
        </p>
      </form>
    </section>
  );
}
