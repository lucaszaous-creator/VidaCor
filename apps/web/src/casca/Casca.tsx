import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Link, Outlet, useNavigate, useRouterState } from '@tanstack/react-router';
import {
  Aviso,
  Botao,
  BotaoDeIcone,
  Carregando,
  ErroTela,
  Icone,
  MenuLateral,
  SemConexao,
  formatarHora,
  type PropsDoLink,
} from '@vidacor/design-system';
import { useState } from 'react';
import simbolo from '../assets/simbolo.png';
import { consultaContingenciaAberta, consultaSessao } from '../consultas';
import { useDependencias } from '../contexto';
import { erroParaTela } from '../erros';
import { ProvedorDeEventos } from '../eventos/EventosAoVivo';
import { FUSO_DA_CLINICA } from '../telas/agenda/grade';
import { itemAtivo, secoesDoMenu } from './menu';

/** Nome do perfil para leitura. Perfil desconhecido aparece como veio do servidor. */
const NOMES_DE_PERFIL: Record<string, string> = {
  recepcao: 'Recepção',
  supervisor_recepcao: 'Supervisão da recepção',
  caixa: 'Caixa',
  tecnico: 'Técnico',
  enfermagem: 'Enfermagem',
  medico: 'Médico',
  faturamento: 'Faturamento',
  coordenacao: 'Coordenação',
  gerente: 'Gerência',
  financeiro: 'Financeiro',
  ti: 'TI',
};

function nomeDoPerfil(perfis: string[]): string {
  const primeiro = perfis[0];
  if (primeiro === undefined) {
    return 'Sem perfil';
  }
  return NOMES_DE_PERFIL[primeiro] ?? primeiro;
}

function LinkDoRoteador({ href, className, children, ...aria }: PropsDoLink) {
  return (
    <Link to={href} className={className} {...aria}>
      {children}
    </Link>
  );
}

const NIVEIS: Record<string, string> = { externo: 'serviços externos', setor: 'um setor', total: 'toda a unidade' };

/**
 * Faixa de contingência aberta no topo de toda tela (R7): quem chega sabe que o atendimento segue no
 * papel. Erro ao consultar não vira faixa nenhuma aqui; a tela de Contingência mostra o erro.
 */
function FaixaDeContingencia() {
  const { api } = useDependencias();
  const aberta = useQuery(consultaContingenciaAberta(api));
  const c = aberta.data;
  if (c === undefined || c === null) {
    return null;
  }
  return (
    <div className="faixa-ctg" role="status">
      <Icone nome="contingencia" tamanho={20} />
      <span className="t14" style={{ flex: 1, minWidth: 0 }}>
        <b>Contingência {c.codigo} aberta</b>, atinge {NIVEIS[c.nivel] ?? c.nivel}: {c.motivo}. Declarada às{' '}
        {formatarHora(c.declarada_em, FUSO_DA_CLINICA)}. O atendimento segue nas fichas em papel.
      </span>
      <Link to="/contingencia" className="btn btn-sm">
        Abrir contingência
      </Link>
    </div>
  );
}

export function Casca() {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const navegar = useNavigate();
  const caminho = useRouterState({ select: (s) => s.location.pathname });
  const sessao = useQuery(consultaSessao(api));
  // Preferência só desta aba, em memória: guardar por usuário depende de rota no núcleo.
  const [recolhido, setRecolhido] = useState(false);
  const [saindo, setSaindo] = useState(false);
  const [erroAoSair, setErroAoSair] = useState<ReturnType<typeof erroParaTela> | null>(null);

  async function sair(): Promise<void> {
    setSaindo(true);
    setErroAoSair(null);
    try {
      await api.sessao.sair();
      queryClient.clear();
      await navegar({ to: '/entrar' });
    } catch (erro) {
      setErroAoSair(erroParaTela(erro));
    } finally {
      setSaindo(false);
    }
  }

  if (sessao.isPending) {
    return (
      <main className="content" style={{ maxWidth: 640, margin: '0 auto' }}>
        <Carregando rotulo="Carregando a sessão" />
      </main>
    );
  }

  if (sessao.isError) {
    const e = erroParaTela(sessao.error);
    if (e.tipo === 'sem-sessao') {
      void navegar({ to: '/entrar' });
      return null;
    }
    const tentar = (
      <Botao pequeno onPress={() => void sessao.refetch()}>
        Tentar de novo
      </Botao>
    );
    return (
      <main className="content" style={{ maxWidth: 640, margin: '0 auto', justifyContent: 'center' }}>
        {e.tipo === 'sem-conexao' ? (
          <SemConexao acao={tentar}>Nada foi alterado. Confira a rede do computador; se continuar, avise a TI.</SemConexao>
        ) : (
          <ErroTela mensagem={e.mensagem} codigoDeSuporte={e.codigoDeSuporte} acao={tentar} />
        )}
      </main>
    );
  }

  const secoes = secoesDoMenu(sessao.data.usuario.perfis);
  const ativo = itemAtivo(caminho, secoes);

  return (
    <ProvedorDeEventos>
      <div className="app">
        <MenuLateral
          secoes={secoes}
          ativo={ativo}
          recolhido={recolhido}
          aoAlternar={() => {
            setRecolhido((r) => !r);
          }}
          marca={{ nome: 'VidaCor', unidade: sessao.data.unidade.nome, href: '/agenda', imagem: simbolo }}
          usuario={{ nome: sessao.data.usuario.nome, papel: nomeDoPerfil(sessao.data.usuario.perfis) }}
          renderizarLink={LinkDoRoteador}
          acaoDoRodape={
            <BotaoDeIcone
              icone="sair"
              rotulo={saindo ? 'Saindo' : 'Sair'}
              isDisabled={saindo}
              onPress={() => void sair()}
            />
          }
        />
        <main className="content">
          <FaixaDeContingencia />
          {erroAoSair !== null && (
            <Aviso nivel="bloqueante" titulo="A sessão não foi encerrada." codigoDeSuporte={erroAoSair.codigoDeSuporte}>
              {erroAoSair.tipo === 'sem-conexao' ? 'Não foi possível falar com o servidor. ' : `${erroAoSair.mensagem} `}
              Tente sair de novo antes de deixar o computador.
            </Aviso>
          )}
          <Outlet />
        </main>
      </div>
    </ProvedorDeEventos>
  );
}
