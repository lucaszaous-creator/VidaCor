import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent, type MouseEvent, type RefObject } from 'react';
import { Header, Keyboard, Menu, MenuItem, MenuSection, Popover, Separator, Text } from 'react-aria-components';

/**
 * Ação de menu como o servidor manda para aquele objeto (docs/19 § 8). A tela não decide quais
 * ações existem: só mostra. Ação que o perfil nunca terá nem chega; indisponível vem com motivo.
 */
export interface AcaoDoMenu {
  id: string;
  rotulo: string;
  atalho?: string;
  estado: 'disponivel' | 'indisponivel';
  /** Obrigatório quando indisponível ("Laudo ainda não assinado"). */
  motivo?: string;
  /** Destrutiva: vai para o fim, em vermelho, separada por linha. */
  perigo?: boolean;
}

export interface CabecalhoDoMenu {
  /** Paciente: nome completo. */
  titulo: string;
  /** Paciente: nascimento; agendamento: horário e exame. */
  subtitulo: string;
}

export interface EstadoDoMenuDeContexto {
  aberto: boolean;
  ancora: RefObject<HTMLElement | null>;
  abrirEm: (elemento: HTMLElement) => void;
  fechar: () => void;
  /** Espalhe no objeto: clique direito, tecla Menu e Shift+F10 abrem o menu. */
  propsDoAlvo: {
    onContextMenu: (ev: MouseEvent<HTMLElement>) => void;
    onKeyDown: (ev: KeyboardEvent<HTMLElement>) => void;
    'aria-haspopup': 'menu';
  };
}

/** Estado do menu de contexto de um objeto. */
export function useMenuDeContexto(): EstadoDoMenuDeContexto {
  const ancora = useRef<HTMLElement | null>(null);
  const [aberto, setAberto] = useState(false);

  const abrirEm = useCallback((elemento: HTMLElement) => {
    ancora.current = elemento;
    elemento.focus();
    setAberto(true);
  }, []);
  const fechar = useCallback(() => {
    setAberto(false);
  }, []);

  // Esc e escolha devolvem o foco ao objeto (docs/19 § 2). Espera o menu sair da tela, porque
  // enquanto ele existe o foco fica preso dentro dele.
  const estavaAberto = useRef(false);
  useEffect(() => {
    if (estavaAberto.current && !aberto) {
      const id = setTimeout(() => {
        ancora.current?.focus();
      }, 0);
      estavaAberto.current = aberto;
      return () => {
        clearTimeout(id);
      };
    }
    estavaAberto.current = aberto;
    return undefined;
  }, [aberto]);

  const propsDoAlvo = useMemo(
    () => ({
      onContextMenu: (ev: MouseEvent<HTMLElement>) => {
        ev.preventDefault();
        abrirEm(ev.currentTarget);
      },
      onKeyDown: (ev: KeyboardEvent<HTMLElement>) => {
        if (ev.key === 'ContextMenu' || (ev.key === 'F10' && ev.shiftKey)) {
          ev.preventDefault();
          abrirEm(ev.currentTarget);
        }
      },
      'aria-haspopup': 'menu' as const,
    }),
    [abrirEm],
  );

  return { aberto, ancora, abrirEm, fechar, propsDoAlvo };
}

export interface PropsDoMenuDeContexto {
  estado: EstadoDoMenuDeContexto;
  cabecalho: CabecalhoDoMenu;
  /** Lista vinda do servidor, na ordem do servidor. */
  acoes: AcaoDoMenu[];
  /** Por que não há ação nenhuma (ex.: "Disponível na próxima entrega"). */
  motivoSemAcoes: string;
  aoEscolher: (id: string) => void;
}

const SEM_ACOES = '__sem-acoes__';

function ItemDoMenu({ acao }: { acao: AcaoDoMenu }) {
  return (
    <MenuItem id={acao.id} textValue={acao.rotulo} className={acao.perigo === true ? 'cm-i danger' : 'cm-i'}>
      <span className="cm-l">
        <Text slot="label">{acao.rotulo}</Text>
        {acao.estado === 'indisponivel' && acao.motivo !== undefined && (
          <Text slot="description" className="cm-why">
            {acao.motivo}
          </Text>
        )}
      </span>
      {acao.atalho !== undefined && <Keyboard className="cm-k">{acao.atalho}</Keyboard>}
    </MenuItem>
  );
}

/**
 * Menu de contexto único (docs/19). Abre por clique direito, tecla Menu, Shift+F10 e pelo botão de
 * mais opções; setas percorrem, Enter executa, Esc fecha e devolve o foco ao objeto.
 */
export function MenuDeContexto({ estado, cabecalho, acoes, motivoSemAcoes, aoEscolher }: PropsDoMenuDeContexto) {
  const comuns = acoes.filter((a) => a.perigo !== true);
  const destrutivas = acoes.filter((a) => a.perigo === true);
  const indisponiveis = acoes.filter((a) => a.estado === 'indisponivel').map((a) => a.id);
  const vazio = acoes.length === 0;

  return (
    <Popover
      triggerRef={estado.ancora}
      isOpen={estado.aberto}
      onOpenChange={(aberto) => {
        if (!aberto) {
          estado.fechar();
        }
      }}
      placement="bottom start"
      className="cm"
    >
      <div className="cm-t">
        <b>{cabecalho.titulo}</b>
        {cabecalho.subtitulo}
      </div>
      <Menu
        aria-label={`Ações de ${cabecalho.titulo}`}
        autoFocus="first"
        disabledKeys={vazio ? [SEM_ACOES] : indisponiveis}
        onAction={(chave) => {
          aoEscolher(String(chave));
          estado.fechar();
        }}
      >
        {vazio ? (
          <MenuItem id={SEM_ACOES} textValue="Nenhuma ação disponível" className="cm-i">
            <span className="cm-l">
              <Text slot="label">Nenhuma ação disponível</Text>
              <Text slot="description" className="cm-why">
                {motivoSemAcoes}
              </Text>
            </span>
          </MenuItem>
        ) : (
          <>
            <MenuSection aria-label="Ações">
              <Header className="sr">Ações</Header>
              {comuns.map((a) => (
                <ItemDoMenu key={a.id} acao={a} />
              ))}
            </MenuSection>
            {destrutivas.length > 0 && <Separator className="cm-sep" />}
            {destrutivas.length > 0 && (
              <MenuSection aria-label="Ações destrutivas">
                <Header className="sr">Ações destrutivas</Header>
                {destrutivas.map((a) => (
                  <ItemDoMenu key={a.id} acao={a} />
                ))}
              </MenuSection>
            )}
          </>
        )}
      </Menu>
    </Popover>
  );
}
