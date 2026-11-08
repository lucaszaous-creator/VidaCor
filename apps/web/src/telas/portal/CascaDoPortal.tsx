import { Link, Outlet } from '@tanstack/react-router';
import { BotaoDeIcone } from '@vidacor/design-system';
import { useState } from 'react';
import simbolo from '../../assets/simbolo.png';

const ESCALAS = [1, 1.15, 1.3, 1.5];

/**
 * Casca do portal do paciente (Portal-Web.dc.html): mesma marca e design system, letra ajustável
 * (R10). A escolha do tamanho fica só em memória: nada vai para o armazenamento do navegador.
 */
export function CascaDoPortal() {
  const [nivel, setNivel] = useState(0);
  const escala = ESCALAS[nivel] ?? 1;
  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column', background: 'var(--bg)', overflow: 'hidden' }}>
      <header className="row" style={{ gap: 12, padding: '10px 24px', background: '#fff', borderBottom: '1px solid var(--line)' }}>
        <Link to="/portal" className="row" style={{ gap: 10, color: 'inherit' }}>
          <img className="sb-logo" src={simbolo} alt="" style={{ width: 44, height: 30, objectFit: 'contain' }} />
          <b style={{ fontSize: 16, color: 'var(--navy)' }}>Meu VidaCor</b>
        </Link>
        <div className="grow" />
        <span className="t13 mute" aria-live="polite">
          Letra {Math.round(escala * 100)}%
        </span>
        <BotaoDeIcone
          icone="letraMenor"
          rotulo="Diminuir a letra"
          isDisabled={nivel === 0}
          onPress={() => {
            setNivel((n) => Math.max(0, n - 1));
          }}
        />
        <BotaoDeIcone
          icone="letraMaior"
          rotulo="Aumentar a letra"
          isDisabled={nivel === ESCALAS.length - 1}
          onPress={() => {
            setNivel((n) => Math.min(ESCALAS.length - 1, n + 1));
          }}
        />
      </header>
      <main className="portal" style={{ flex: '1 1 auto', minHeight: 0, overflowY: 'auto', padding: '20px 24px' }}>
        <div style={{ zoom: escala, maxWidth: 960, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
          <Outlet />
        </div>
      </main>
    </div>
  );
}
