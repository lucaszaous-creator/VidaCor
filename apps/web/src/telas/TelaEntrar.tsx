import { useQuery } from '@tanstack/react-query';
import { Botao, Chip } from '@vidacor/design-system';
import { consultaSaude } from '../consultas';
import { useDependencias } from '../contexto';

/**
 * Entrada (Login.dc.html). Usuário, senha e segundo fator ficam no login da clínica (OIDC no
 * servidor): esta tela só leva até lá. Nenhuma senha ou token passa por aqui.
 */
export function TelaEntrar() {
  const { api, irPara } = useDependencias();
  const saude = useQuery(consultaSaude(api));

  let situacao;
  if (saude.isPending) {
    situacao = <Chip cor="cinza" texto="Conferindo o servidor local" />;
  } else if (saude.isSuccess) {
    situacao = <Chip cor="verde" ponto texto="Servidor local conectado" />;
  } else {
    situacao = <Chip cor="ambar" ponto texto="Servidor local sem resposta" />;
  }

  return (
    <div style={{ height: '100vh', overflow: 'hidden', background: 'var(--bg)', display: 'flex', flexDirection: 'column' }}>
      <header className="row" style={{ gap: 16, padding: '14px 28px', borderBottom: '1px solid var(--line)', background: '#fff' }}>
        <span
          aria-hidden="true"
          style={{
            width: 36,
            height: 36,
            borderRadius: 10,
            background: 'var(--navy)',
            color: '#fff',
            display: 'inline-flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} width={20} height={20} focusable="false">
            <path d="M3 12h4l2-5 4 10 2-5h6" />
          </svg>
        </span>
        <div style={{ fontSize: 17, fontWeight: 700, color: 'var(--navy)' }}>VidaCor Imagem</div>
        <div className="grow" />
        <div role="status">{situacao}</div>
      </header>
      <main style={{ flex: '1 1 auto', minHeight: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 24 }}>
        <section
          className="card"
          aria-labelledby="entrar-titulo"
          style={{ width: 440, maxWidth: '100%', padding: 28, boxShadow: '0 4px 16px rgba(15,23,42,.06)' }}
        >
          <h1 id="entrar-titulo" style={{ fontSize: 24, fontWeight: 650, marginBottom: 8 }}>
            Entrar
          </h1>
          <p className="t14 mute" style={{ margin: '0 0 20px', lineHeight: 1.5 }}>
            Usuário, senha e código do celular são pedidos no login da clínica.
          </p>
          <Botao
            variante="principal"
            larguraTotal
            onPress={() => {
              irPara(api.sessao.urlDeEntrada());
            }}
          >
            Entrar
          </Botao>
          {saude.isError && (
            <p className="t13" style={{ margin: '14px 0 0', color: 'var(--amber)', lineHeight: 1.5 }} role="alert">
              O servidor local não respondeu. Confira se este computador está na rede da clínica; se continuar, avise a TI.
            </p>
          )}
        </section>
      </main>
    </div>
  );
}
