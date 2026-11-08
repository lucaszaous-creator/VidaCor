import { useQuery } from '@tanstack/react-query';
import { useRouter } from '@tanstack/react-router';
import type { ObjetoDoMenu } from '@vidacor/api-client';
import { MenuDeContexto, type AcaoDoMenu, type CabecalhoDoMenu, type EstadoDoMenuDeContexto } from '@vidacor/design-system';
import { useDependencias } from '../contexto';

interface Props {
  estado: EstadoDoMenuDeContexto;
  objeto: ObjetoDoMenu;
  id: string;
  /** Cabeçalho mostrado enquanto o servidor responde (nome e nascimento já na tela). */
  cabecalho: CabecalhoDoMenu;
}

/**
 * Menu de contexto alimentado pelo servidor (docs/19 § 8). A lista é pedida quando o menu abre: o
 * perfil e o estado do objeto decidem o que aparece; a tela só mostra e leva à tela que faz a ação
 * (a mesma rota da ação fora do menu). "Copiar nome e nascimento" é a única ação local.
 */
export function MenuDoServidor({ estado, objeto, id, cabecalho }: Props) {
  const { api } = useDependencias();
  const router = useRouter();
  const menu = useQuery({ queryKey: ['menu', objeto, id], queryFn: () => api.menu.de(objeto, id), enabled: estado.aberto, staleTime: 0 });
  const acoes: AcaoDoMenu[] = (menu.data?.acoes ?? []).map((a) => ({
    id: a.id,
    rotulo: a.rotulo,
    estado: a.estado,
    ...(typeof a.motivo === 'string' ? { motivo: a.motivo } : {}),
    perigo: a.perigo,
  }));
  const motivo = menu.isError
    ? 'Não foi possível carregar as ações agora. Nada foi alterado.'
    : menu.isPending
      ? 'Carregando as ações deste item.'
      : 'Seu perfil não tem ações para este item.';
  const atual = menu.data?.cabecalho ?? cabecalho;

  return (
    <MenuDeContexto
      estado={estado}
      cabecalho={atual}
      acoes={acoes}
      motivoSemAcoes={motivo}
      aoEscolher={(escolhida) => {
        const acao = menu.data?.acoes.find((a) => a.id === escolhida);
        if (acao?.estado !== 'disponivel') return;
        if (acao.id === 'copiar') {
          const nascimento = atual.subtitulo.split(',')[0] ?? '';
          // A área de transferência pode não existir (navegador antigo, página sem HTTPS).
          if ('clipboard' in navigator) void navigator.clipboard.writeText(`${atual.titulo}, ${nascimento.toLowerCase()}`);
          return;
        }
        if (typeof acao.destino === 'string' && acao.destino.startsWith('/') && !acao.destino.startsWith('//')) {
          router.history.push(acao.destino);
        }
      }}
    />
  );
}
