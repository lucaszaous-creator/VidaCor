import { useState, type ReactNode } from 'react';
import { Avatar } from './Avatar';
import { Icone, type NomeDoIcone } from './Icone';

/** Cores de módulo do vc10 (`.ni` com `k-<cor>`), sempre a mesma cor por módulo em todo o sistema. */
export type CorDoModulo =
  | 'navy'
  | 'blue'
  | 'teal'
  | 'violet'
  | 'green'
  | 'amber'
  | 'rose'
  | 'indigo'
  | 'pink'
  | 'orange'
  | 'gray';

export interface ItemDoMenu {
  id: string;
  rotulo: string;
  href: string;
  icone: NomeDoIcone;
  cor: CorDoModulo;
  /** Contador vindo do servidor. Sem dado, não mostra (nunca "0" inventado). */
  contador?: number;
  /** Selo de alerta vindo do servidor, com o texto para leitor de tela. */
  alerta?: { numero: number; rotulo: string };
}

export interface SecaoDoMenu {
  id: string;
  titulo: string;
  itens: ItemDoMenu[];
}

export interface PropsDoLink {
  href: string;
  className: string;
  'aria-label'?: string;
  'aria-current'?: 'page';
  children: ReactNode;
}

export interface PropsDoMenuLateral {
  secoes: SecaoDoMenu[];
  /** Id do item da tela atual. */
  ativo: string;
  recolhido: boolean;
  aoAlternar: () => void;
  marca: { nome: string; unidade: string; href: string; imagem?: string };
  usuario: { nome: string; papel: string };
  /** Ação do rodapé (por exemplo, sair), já com rótulo acessível. */
  acaoDoRodape?: ReactNode;
  /** Troca o `<a>` pelo link do roteador da aplicação. */
  renderizarLink?: (props: PropsDoLink) => ReactNode;
}

function LinkPadrao(props: PropsDoLink) {
  return <a {...props} />;
}

function textoDaDica(item: ItemDoMenu): string {
  if (item.alerta !== undefined) {
    return `${item.rotulo}, ${item.alerta.rotulo}`;
  }
  if (item.contador !== undefined) {
    return `${item.rotulo}, ${String(item.contador)}`;
  }
  return item.rotulo;
}

/**
 * Menu lateral único (Sidebar.dc.html). Recolhível entre completo e só ícones; recolhido, cada ícone
 * tem dica com nome e contador e os grupos viram separadores. Só o grupo da tela atual vem aberto.
 */
export function MenuLateral({ secoes, ativo, recolhido, aoAlternar, marca, usuario, acaoDoRodape, renderizarLink }: PropsDoMenuLateral) {
  const Link = renderizarLink ?? LinkPadrao;
  const secaoAtiva = secoes.find((s) => s.itens.some((i) => i.id === ativo))?.id;
  const [abertas, setAbertas] = useState<Record<string, boolean>>({});
  const estaAberta = (id: string): boolean => abertas[id] ?? id === secaoAtiva;

  const logo =
    marca.imagem === undefined ? null : (
      <img className="sb-logo" src={marca.imagem} alt="" style={{ width: 44, height: 30, objectFit: 'contain' }} />
    );

  return (
    <div className="side">
      <nav
        aria-label="Menu principal"
        className={recolhido ? 'sb sb-mini' : 'sb'}
        style={{ display: 'flex', flexDirection: 'column', gap: 2, background: 'var(--bg)', padding: '0 0 8px' }}
      >
        {!recolhido && (
          <>
            <div className="row" style={{ gap: 6 }}>
              <Link href={marca.href} className="sb-marca">
                <span style={{ flex: 1, minWidth: 0, display: 'flex', alignItems: 'center', gap: 12, padding: '8px 6px' }}>
                  {logo}
                  <span style={{ flex: 1, minWidth: 0, lineHeight: 1.25 }}>
                    <b style={{ display: 'block', fontSize: 15, fontWeight: 650, color: 'var(--navy)', whiteSpace: 'nowrap' }}>
                      {marca.nome}
                    </b>
                    <span style={{ fontSize: 13, color: 'var(--mute)' }}>{marca.unidade}</span>
                  </span>
                </span>
              </Link>
              <button type="button" className="sb-tg" aria-expanded="true" aria-label="Recolher menu" title="Recolher menu" onClick={aoAlternar}>
                <Icone nome="recolher" tamanho={20} traco={1.8} />
              </button>
            </div>
            {secoes.map((secao) => {
              const aberta = estaAberta(secao.id);
              return (
                <div key={secao.id} style={{ display: 'contents' }}>
                  <button
                    type="button"
                    className="sec sec-b"
                    aria-expanded={aberta}
                    onClick={() => {
                      setAbertas((a) => ({ ...a, [secao.id]: !aberta }));
                    }}
                  >
                    {secao.titulo}
                    <span style={{ marginLeft: 'auto', display: 'inline-flex' }}>
                      <Icone nome={aberta ? 'abaixo' : 'proximo'} tamanho={14} traco={2.2} />
                    </span>
                  </button>
                  {aberta &&
                    secao.itens.map((item) => (
                      <Link
                        key={item.id}
                        href={item.href}
                        className={item.id === ativo ? 'nv on' : 'nv'}
                        aria-current={item.id === ativo ? 'page' : undefined}
                      >
                        <span className={`ni k-${item.cor}`}>
                          <Icone nome={item.icone} tamanho={20} traco={1.8} />
                        </span>
                        <span>{item.rotulo}</span>
                        {item.contador !== undefined && <span className="ct">{item.contador}</span>}
                        {item.alerta !== undefined && (
                          <span className="al" aria-label={item.alerta.rotulo}>
                            {item.alerta.numero}
                          </span>
                        )}
                      </Link>
                    ))}
                </div>
              );
            })}
            <div className="row" style={{ marginTop: 12, padding: '10px 8px 0', borderTop: '1px solid #E3E7ED' }}>
              <Avatar nome={usuario.nome} />
              <span style={{ flex: 1, minWidth: 0, lineHeight: 1.3 }}>
                <b className="t15 clamp1" style={{ display: 'block', fontWeight: 600 }}>
                  {usuario.nome}
                </b>
                <span className="t13 mute">{usuario.papel}</span>
              </span>
              {acaoDoRodape}
            </div>
          </>
        )}
        {recolhido && (
          <>
            <Link href={marca.href} className="nv-i" aria-label={`${marca.nome}, início`}>
              {logo}
            </Link>
            <button type="button" className="sb-tg nv-i" aria-expanded="false" aria-label="Expandir menu" onClick={aoAlternar} style={{ marginTop: 6 }}>
              <Icone nome="expandir" tamanho={20} traco={1.8} />
              <span className="tip">Expandir menu</span>
            </button>
            {secoes.map((secao) => (
              <div key={secao.id} style={{ display: 'contents' }}>
                <div className="sb-sep" role="separator" aria-label={secao.titulo} />
                {secao.itens.map((item) => (
                  <Link
                    key={item.id}
                    href={item.href}
                    className={item.id === ativo ? 'nv-i on' : 'nv-i'}
                    aria-label={textoDaDica(item)}
                    aria-current={item.id === ativo ? 'page' : undefined}
                  >
                    <span className={`ni k-${item.cor}`}>
                      <Icone nome={item.icone} tamanho={20} traco={1.8} />
                    </span>
                    {item.alerta !== undefined && <span className="al">{item.alerta.numero}</span>}
                    <span className="tip" aria-hidden="true">
                      {textoDaDica(item)}
                    </span>
                  </Link>
                ))}
              </div>
            ))}
            <div className="sb-sep" />
            <div className="nv-i" role="group" aria-label={`${usuario.nome}, ${usuario.papel}`}>
              <Avatar nome={usuario.nome} />
              <span className="tip" aria-hidden="true">
                {usuario.nome}, {usuario.papel}
              </span>
            </div>
            {acaoDoRodape}
          </>
        )}
      </nav>
    </div>
  );
}
