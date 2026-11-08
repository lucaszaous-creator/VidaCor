import type {
  AcessoDeEmergencia,
  AgendaFase1,
  AgendamentoCriado,
  AtendimentoDetalhadoFase1,
  AtendimentoFase1,
  AutorizacaoDeDesconto,
  Caixa,
  Contingencia,
  CorrecaoCadastro,
  Estorno,
  FilaDaSala,
  Guia,
  HorarioLivre,
  ItemDaFilaDeLaudos,
  ItemDoPortal,
  Laudo,
  LaudoDoPortal,
  PedidoAcessoEmergencia,
  PedidoAgendamento,
  PedidoAssinatura,
  PedidoAutorizacaoDesconto,
  PedidoBipe,
  PedidoCheckin,
  PedidoConduta,
  PedidoContingencia,
  PedidoConversarAntes,
  PedidoCorrecaoCadastro,
  PedidoEstorno,
  PedidoFicha,
  PedidoGuia,
  PedidoRascunho,
  PedidoRecebimento,
  PedidoRecibo,
  PedidoReimpressao,
  PedidoRetificacao,
  PedidoTriagem,
  Recebimento,
  ReciboLancado,
  Reimpressao,
  ResultadoDaFicha,
  ResultadoDoBipe,
  Triagem,
  Divergencia,
  EstudoPendente,
  LinkCriado,
  PedidoConciliacao,
  PedidoNumeros,
} from './contrato-fase1';
import type { ConsultaAgenda, PacienteResumo, SalaDaUnidade, Saude, Sessao } from './contrato-m0';
import type { MenuDoObjeto, ObjetoDoMenu } from './contrato-menu';
import type { CadeiaDoAtendimento, DocumentoBaixado, PedidoDossie } from './contrato-lastro';
import type { Indicadores } from './contrato-indicadores';
import type { ChamadaNoPainel, Painel, PedidoDeChamada } from './contrato-painel';
import type { CatalogoDoOrcamento, NovoOrcamento, Orcamento } from './contrato-orcamento';
import type { IntegracaoVista } from './contrato-configuracoes';
import type { VisaoDoPaciente } from './contrato-paciente';
import type { InicioDoDia } from './contrato-inicio';
import type { BloqueioDeSala, OfertaDeVaga, PedidoBloqueio, PedidoDeEspera, PedidoInclusao } from './contrato-espera';
import type { AparelhoCadastrado, MudancaDeAparelho, NovoAparelho, PainelDosAparelhos } from './contrato-aparelhos';
import type { DecisaoDePerfil, ExameExterno, ImportacaoExterna, MudancaDePerfil, NovoUsuario, UsuarioAdministrado } from './contrato-usuarios';
import type { ContaBancaria, NotaFiscal, NovoTitulo, RepasseDoMedico, RepasseGerado, ResumoFinanceiro, Titulo } from './contrato-financeiro';
import type { ComunicacaoDoPaciente, LembretesEnfileirados, PreferenciaWhatsapp } from './contrato-comunicacao';
import type {
  AutorizacaoDetalhe,
  AutorizacaoResumo,
  NovoPedidoAutorizacao,
  PedidoProtocolo,
  RespostaDoPortal,
  SituacaoDaAutorizacao,
} from './contrato-autorizacao';
import type {
  ContaDetalhe,
  ContaResumo,
  ContratoDeConvenio,
  Glosa,
  LoteDetalhe,
  LoteTiss,
  PainelAuditoria,
  PedidoCorrecaoConta,
  PedidoDemonstrativo,
  PedidoLote,
  PedidoRecurso,
  ResultadoConferencia,
  SituacaoDaConta,
} from './contrato-faturamento';
import type { ContrasteAplicado, EntradaDeEstoque, ItemDeEstoque, MovimentoFeito, RastreioDoLote } from './contrato-estoque';
import type { AceiteDoConvite, ConviteEmitido, EntradaNoPortal, NovoPaciente, PacienteCadastrado, PossivelDuplicado, SessaoDoPortal, SituacaoDoConvite } from './contrato-portal';
import type { AchadoEstruturado, AchadosDoLaudo, NovoAchado } from './contrato-laudo-estruturado';
import type { AtendimentoAReceber, Decisao, EstornoDetalhado, NovoPedidoDesconto, PedidoDesconto, SituacaoDoEstorno, SituacaoDoPedidoDeDesconto } from './contrato-pendencias';
import { ErroDaApi, ErroSemConexao, ehProblemaHttp, problemaInesperado } from './erros';

export interface OpcoesDoCliente {
  /** Origem do núcleo. Padrão: a origem da própria página (o Vite encaminha `/v1` ao núcleo). */
  base?: string;
  /** Implementação de `fetch` (testes). */
  fetch?: typeof globalThis.fetch;
  /** Quantas vezes reenviar uma escrita quando a rede falha, sempre com a mesma chave. Padrão: 2. */
  retentativasDeRede?: number;
  /** Espera entre retentativas (testes trocam por espera instantânea). */
  esperar?: (ms: number) => Promise<void>;
  /**
   * Estação de trabalho (`GUICHE-1`, `SALA-1`), gravada no lastro pelo servidor (docs/30 § 0).
   * Só letras, números e hífen, até 32 caracteres; valor fora disso não é enviado.
   */
  estacao?: string;
}

/** Opções de uma escrita que cria algo (G3). */
export interface OpcoesDeEscrita {
  /**
   * UUID gerado UMA vez por operação de negócio (ao abrir o formulário) e reaproveitado em toda
   * retentativa, para que repetir não duplique. Nunca dado pessoal.
   */
  idempotencyKey: string;
}

type Metodo = 'GET' | 'POST' | 'PUT';

interface Pedido {
  metodo: Metodo;
  caminho: string;
  corpo?: unknown;
  escrita?: boolean;
  idempotencyKey?: string;
}

const ESPERA_BASE_MS = 500;
const ESTACAO_VALIDA = /^[A-Za-z0-9-]{1,32}$/;

function seg(valor: string): string {
  return encodeURIComponent(valor);
}

function esperarDeVerdade(ms: number): Promise<void> {
  return new Promise((resolver) => {
    setTimeout(resolver, ms);
  });
}

/** Gera a chave de idempotência de uma operação de negócio. */
export function novaChaveDeIdempotencia(): string {
  return globalThis.crypto.randomUUID();
}

export type ClienteVidaCor = ReturnType<typeof criarCliente>;

/**
 * Cliente fino do núcleo. Sessão em cookie HttpOnly (`credentials: 'include'`); nenhum token passa
 * por aqui nem é guardado no navegador. O CSRF vem de `GET /v1/sessao` e fica só nesta closure.
 */
export function criarCliente(opcoes: OpcoesDoCliente = {}) {
  const base = opcoes.base ?? globalThis.location.origin;
  const buscar = opcoes.fetch ?? globalThis.fetch.bind(globalThis);
  const retentativas = opcoes.retentativasDeRede ?? 2;
  const esperar = opcoes.esperar ?? esperarDeVerdade;
  const estacao = opcoes.estacao !== undefined && ESTACAO_VALIDA.test(opcoes.estacao) ? opcoes.estacao : null;
  let csrf: string | null = null;

  function url(caminho: string): string {
    return new URL(caminho, base).toString();
  }

  async function lerRespostaComSituacao(resposta: Response): Promise<{ status: number; corpo: unknown }> {
    if (resposta.ok) {
      if (resposta.status === 204) {
        return { status: 204, corpo: undefined };
      }
      return { status: resposta.status, corpo: (await resposta.json()) as unknown };
    }
    let corpo: unknown;
    try {
      corpo = await resposta.json();
    } catch {
      corpo = null;
    }
    if (resposta.status === 401) {
      csrf = null;
    }
    throw new ErroDaApi(ehProblemaHttp(corpo) ? corpo : problemaInesperado(resposta.status));
  }

  async function enviarUmaVez(pedido: Pedido, cabecalhos: Record<string, string>): Promise<Response> {
    try {
      return await buscar(url(pedido.caminho), {
        method: pedido.metodo,
        credentials: 'include',
        cache: 'no-store',
        headers: cabecalhos,
        body: pedido.corpo === undefined ? null : JSON.stringify(pedido.corpo),
      });
    } catch (causa) {
      throw new ErroSemConexao(causa);
    }
  }

  async function pedir<T>(pedido: Pedido): Promise<T> {
    const r = await pedirComSituacao(pedido);
    return r.corpo as T;
  }

  async function pedirComSituacao(pedido: Pedido): Promise<{ status: number; corpo: unknown }> {
    const cabecalhos: Record<string, string> = { Accept: 'application/json' };
    if (estacao !== null) {
      cabecalhos['X-Estacao'] = estacao;
    }
    if (pedido.corpo !== undefined) {
      cabecalhos['Content-Type'] = 'application/json';
    }
    if (pedido.escrita === true) {
      if (csrf === null) {
        await obterSessao();
      }
      if (csrf !== null) {
        cabecalhos['X-CSRF-Token'] = csrf;
      }
    }
    if (pedido.idempotencyKey !== undefined) {
      cabecalhos['Idempotency-Key'] = pedido.idempotencyKey;
    }
    // Só escrita com chave de idempotência é reenviada sozinha: a chave garante que não duplica.
    const maximo = pedido.idempotencyKey === undefined ? 0 : retentativas;
    for (let tentativa = 0; ; tentativa += 1) {
      try {
        const resposta = await enviarUmaVez(pedido, cabecalhos);
        return await lerRespostaComSituacao(resposta);
      } catch (erro) {
        if (erro instanceof ErroSemConexao && tentativa < maximo) {
          await esperar(ESPERA_BASE_MS * 2 ** tentativa);
          continue;
        }
        throw erro;
      }
    }
  }

  async function obterSessao(): Promise<Sessao> {
    const sessao = await pedir<Sessao>({ metodo: 'GET', caminho: '/v1/sessao' });
    csrf = sessao.csrf;
    return sessao;
  }

  /** Escrita que cria registro: CSRF e `Idempotency-Key` (G3). */
  function criar<T>(metodo: Metodo, caminho: string, corpo: unknown, opcoesDeEscrita: OpcoesDeEscrita): Promise<T> {
    return pedir<T>({ metodo, caminho, corpo, escrita: true, idempotencyKey: opcoesDeEscrita.idempotencyKey });
  }

  /** Escrita sem chave (atualização idempotente por natureza, como salvar rascunho). */
  function escrever<T>(metodo: Metodo, caminho: string, corpo: unknown): Promise<T> {
    return pedir<T>({ metodo, caminho, corpo, escrita: true });
  }

  /** Escrita do portal do paciente: sessão própria (cookie do portal), sem o CSRF da equipe. */
  async function noPortal(caminho: string, corpo: unknown): Promise<void> {
    await pedir<undefined>({ metodo: 'POST', caminho, corpo });
  }

  function ler<T>(caminho: string): Promise<T> {
    return pedir<T>({ metodo: 'GET', caminho });
  }

  /**
   * Baixa um documento binário (PDF). O SHA-256 dos bytes vem do servidor no cabeçalho
   * `X-Documento-Sha256` e a emissão fica registrada lá; nada é guardado no navegador.
   * Erro do servidor chega como `ErroDaApi`, igual às outras rotas.
   */
  async function baixar(pedido: Pedido): Promise<DocumentoBaixado> {
    const cabecalhos: Record<string, string> = { Accept: 'application/pdf, application/problem+json' };
    if (estacao !== null) {
      cabecalhos['X-Estacao'] = estacao;
    }
    if (pedido.corpo !== undefined) {
      cabecalhos['Content-Type'] = 'application/json';
    }
    if (pedido.escrita === true) {
      if (csrf === null) {
        await obterSessao();
      }
      if (csrf !== null) {
        cabecalhos['X-CSRF-Token'] = csrf;
      }
    }
    const resposta = await enviarUmaVez(pedido, cabecalhos);
    if (!resposta.ok) {
      await lerRespostaComSituacao(resposta);
    }
    const disposicao = resposta.headers.get('Content-Disposition') ?? '';
    const nome = /filename="([^"]+)"/.exec(disposicao)?.[1] ?? 'documento.pdf';
    return {
      conteudo: await resposta.blob(),
      nomeDoArquivo: nome,
      sha256: resposta.headers.get('X-Documento-Sha256') ?? '',
      codigoDaEmissao: resposta.headers.get('X-Documento-Codigo') ?? '',
    };
  }

  return {
    saude: (): Promise<Saude> => ler<Saude>('/v1/saude'),

    sessao: {
      obter: obterSessao,
      /** Endereço que inicia a entrada (OIDC no servidor). A página navega para ele. */
      urlDeEntrada: (): string => url('/v1/sessao/entrar'),
      /**
       * Reautenticação de ato sensível (SEC-AUT-5, docs/30 § 0). `voltar` é um caminho da própria
       * aplicação para onde o servidor devolve; outra origem nunca é aceita (vira `/`).
       */
      urlDeReautenticacao: (voltar: string): string => {
        const destino = voltar.startsWith('/') && !voltar.startsWith('//') ? voltar : '/';
        return url(`/v1/sessao/reautenticar?${new URLSearchParams({ voltar: destino }).toString()}`);
      },
      sair: async (): Promise<void> => {
        await pedir<undefined>({ metodo: 'POST', caminho: '/v1/sessao/sair', escrita: true });
        csrf = null;
      },
    },

    salas: {
      listar: (): Promise<SalaDaUnidade[]> => ler<SalaDaUnidade[]>('/v1/salas'),
      /** Fila do dia da sala (ato `sala.ver`). */
      fila: (salaId: string): Promise<FilaDaSala> => ler<FilaDaSala>(`/v1/salas/${seg(salaId)}/fila`),
      /** Bipe da pulseira (leitor = teclado + Enter). Quem decide se libera é o servidor. */
      bipar: (salaId: string, corpo: PedidoBipe, o: OpcoesDeEscrita): Promise<ResultadoDoBipe> =>
        criar<ResultadoDoBipe>('POST', `/v1/salas/${seg(salaId)}/bipes`, corpo, o),
      horariosLivres: (salaId: string, data: string): Promise<HorarioLivre[]> =>
        ler<HorarioLivre[]>(`/v1/salas/${seg(salaId)}/horarios-livres?${new URLSearchParams({ data }).toString()}`),
    },

    agenda: {
      obter: (consulta: ConsultaAgenda): Promise<AgendaFase1> => {
        const parametros = new URLSearchParams({ data: consulta.data });
        if (consulta.sala_id !== undefined && consulta.sala_id !== '') {
          parametros.set('sala_id', consulta.sala_id);
        }
        return ler<AgendaFase1>(`/v1/agenda?${parametros.toString()}`);
      },
    },

    atendimentos: {
      /** Check-in. O servidor confere os dois identificadores e gera senha, pulseira e códigos (G1). */
      criar: (corpo: PedidoCheckin, o: OpcoesDeEscrita): Promise<AtendimentoFase1> =>
        criar<AtendimentoFase1>('POST', '/v1/atendimentos', corpo, o),
      obter: (id: string): Promise<AtendimentoDetalhadoFase1> =>
        ler<AtendimentoDetalhadoFase1>(`/v1/atendimentos/${seg(id)}`),
      /** Triagem com contraste. O servidor calcula a TFG e o resultado (FC-TRI-07). */
      registrarTriagem: (id: string, corpo: PedidoTriagem, o: OpcoesDeEscrita): Promise<Triagem> =>
        criar<Triagem>('POST', `/v1/atendimentos/${seg(id)}/triagem`, corpo, o),
      /** Conduta médica (reautenticação). */
      registrarConduta: (id: string, corpo: PedidoConduta, o: OpcoesDeEscrita): Promise<Triagem> =>
        criar<Triagem>('POST', `/v1/atendimentos/${seg(id)}/triagem/conduta`, corpo, o),
      /** Chama (ou rechama) a senha: recepção ao guichê, técnico à sala (o servidor decide o texto). */
      chamar: (id: string, corpo: PedidoDeChamada, o: OpcoesDeEscrita): Promise<ChamadaNoPainel> =>
        criar<ChamadaNoPainel>('POST', `/v1/atendimentos/${seg(id)}/chamadas`, corpo, o),
    },

    /** Painel da TV de chamada (dispositivo TV, recepção e gerência). */
    painelDeChamada: (): Promise<Painel> => ler<Painel>('/v1/painel-de-chamada'),

    /** O dia da unidade (recepção e gerência). */
    inicio: (): Promise<InicioDoDia> => ler<InicioDoDia>('/v1/inicio'),

    /** Situação das integrações externas (TI e gerência); nunca traz valor de configuração. */
    integracoes: (): Promise<IntegracaoVista[]> => ler<IntegracaoVista[]>('/v1/configuracoes/integracoes'),

    /** Lista de espera e ofertas de vaga (recepção e gerência). */
    espera: {
      listar: (): Promise<PedidoDeEspera[]> => ler<PedidoDeEspera[]>('/v1/lista-espera'),
      incluir: (corpo: PedidoInclusao, o: OpcoesDeEscrita): Promise<PedidoDeEspera> => criar<PedidoDeEspera>('POST', '/v1/lista-espera', corpo, o),
      sair: (id: string, motivo: string, o: OpcoesDeEscrita): Promise<unknown> => criar<unknown>('POST', `/v1/lista-espera/${seg(id)}/saida`, { motivo }, o),
      aceitar: (ofertaId: string, nome: string, nascimento: string, o: OpcoesDeEscrita): Promise<OfertaDeVaga> =>
        criar<OfertaDeVaga>('POST', `/v1/ofertas/${seg(ofertaId)}/aceite`, { confirmacao: { nome, nascimento } }, o),
      recusar: (ofertaId: string, motivo: string, o: OpcoesDeEscrita): Promise<OfertaDeVaga> =>
        criar<OfertaDeVaga>('POST', `/v1/ofertas/${seg(ofertaId)}/recusa`, { motivo }, o),
    },

    /** Bloqueios de sala e horário (recepção e gerência). */
    bloqueios: {
      listar: (de: string, ate: string): Promise<BloqueioDeSala[]> =>
        ler<BloqueioDeSala[]>(`/v1/bloqueios?de=${encodeURIComponent(de)}&ate=${encodeURIComponent(ate)}`),
      criar: (corpo: PedidoBloqueio, o: OpcoesDeEscrita): Promise<BloqueioDeSala[]> => criar<BloqueioDeSala[]>('POST', '/v1/bloqueios', corpo, o),
      retirar: (id: string, motivo: string, o: OpcoesDeEscrita): Promise<BloqueioDeSala> =>
        criar<BloqueioDeSala>('POST', `/v1/bloqueios/${seg(id)}/retirada`, { motivo }, o),
    },

    /** Aparelhos e integração DICOM (TI e gerência). */
    aparelhos: {
      painel: (): Promise<PainelDosAparelhos> => ler<PainelDosAparelhos>('/v1/aparelhos'),
      cadastrar: (corpo: NovoAparelho, o: OpcoesDeEscrita): Promise<AparelhoCadastrado> => criar<AparelhoCadastrado>('POST', '/v1/aparelhos', corpo, o),
      mudar: (id: string, corpo: MudancaDeAparelho, o: OpcoesDeEscrita): Promise<AparelhoCadastrado> =>
        criar<AparelhoCadastrado>('POST', `/v1/aparelhos/${seg(id)}/mudancas`, corpo, o),
    },

    /** Orçamento do particular: o servidor calcula preço, pacote e desconto (G1). */
    orcamentos: {
      catalogo: (): Promise<CatalogoDoOrcamento> => ler<CatalogoDoOrcamento>('/v1/orcamentos/catalogo'),
      listar: (pacienteId?: string): Promise<Orcamento[]> =>
        ler<Orcamento[]>(pacienteId === undefined ? '/v1/orcamentos' : `/v1/orcamentos?paciente_id=${encodeURIComponent(pacienteId)}`),
      emitir: (corpo: NovoOrcamento, o: OpcoesDeEscrita): Promise<Orcamento> => criar<Orcamento>('POST', '/v1/orcamentos', corpo, o),
      cancelar: (id: string, motivo: string, o: OpcoesDeEscrita): Promise<Orcamento> =>
        criar<Orcamento>('POST', `/v1/orcamentos/${seg(id)}/cancelamento`, { motivo }, o),
    },

    etiquetas: {
      reimprimir: (codigo: string, corpo: PedidoReimpressao, o: OpcoesDeEscrita): Promise<Reimpressao> =>
        criar<Reimpressao>('POST', `/v1/etiquetas/${seg(codigo)}/reimpressao`, corpo, o),
    },

    pacientes: {
      /** Até 20 pacientes com nome e nascimento. O texto vai na consulta à API, nunca no endereço da página. */
      buscar: (texto: string): Promise<PacienteResumo[]> =>
        ler<PacienteResumo[]>(`/v1/pacientes?${new URLSearchParams({ busca: texto }).toString()}`),
      /** Correção de cadastro com motivo (reautenticação). */
      corrigir: (id: string, corpo: PedidoCorrecaoCadastro, o: OpcoesDeEscrita): Promise<CorrecaoCadastro> =>
        criar<CorrecaoCadastro>('POST', `/v1/pacientes/${seg(id)}/correcao`, corpo, o),
      /** Possíveis duplicados (nome e nascimento parecidos, mesmo CPF) antes de salvar. */
      possiveisDuplicados: (nome: string, nascimento: string, cpf?: string): Promise<PossivelDuplicado[]> =>
        ler<PossivelDuplicado[]>(
          `/v1/pacientes/possiveis-duplicados?${new URLSearchParams({ nome, nascimento, ...(cpf === undefined || cpf === '' ? {} : { cpf }) }).toString()}`,
        ),
      /** Cadastro; `409 possivel-duplicado` traz `candidatos` quando ainda não conferido. */
      cadastrar: (corpo: NovoPaciente, o: OpcoesDeEscrita): Promise<PacienteCadastrado> => criar<PacienteCadastrado>('POST', '/v1/pacientes', corpo, o),
      /** Paciente 360: identificação (CPF mascarado), próximos e histórico; leitura auditada. */
      visao: (id: string): Promise<VisaoDoPaciente> => ler<VisaoDoPaciente>(`/v1/pacientes/${seg(id)}/visao`),
      examesExternos: (id: string): Promise<ExameExterno[]> => ler<ExameExterno[]>(`/v1/pacientes/${seg(id)}/exames-externos`),
      /** Importação de exame externo (só leitura) depois da conferência de identidade. */
      importarExameExterno: (id: string, corpo: ImportacaoExterna, o: OpcoesDeEscrita): Promise<ExameExterno> =>
        criar<ExameExterno>('POST', `/v1/pacientes/${seg(id)}/exames-externos`, corpo, o),
      /** Convite do portal por e-mail (endereço do cadastro); `422 paciente-sem-email` sem e-mail. */
      convidarParaPortal: (id: string, o: OpcoesDeEscrita): Promise<ConviteEmitido> =>
        criar<ConviteEmitido>('POST', `/v1/pacientes/${seg(id)}/convites-portal`, {}, o),
    },

    guias: {
      registrar: (corpo: PedidoGuia, o: OpcoesDeEscrita): Promise<Guia> => criar<Guia>('POST', '/v1/guias', corpo, o),
    },

    agendamentos: {
      criar: (corpo: PedidoAgendamento, o: OpcoesDeEscrita): Promise<AgendamentoCriado> =>
        criar<AgendamentoCriado>('POST', '/v1/agendamentos', corpo, o),
      /** Cancela com motivo; a vaga vai à lista de espera. Com guia consumida, `409 guia-consumida`. */
      cancelar: (id: string, motivo: string, o: OpcoesDeEscrita): Promise<unknown> =>
        criar<unknown>('POST', `/v1/agendamentos/${seg(id)}/cancelamento`, { motivo }, o),
      /** Confirma a reserva provisória com a guia entregue (FC-AGD-09). */
      vincularGuia: (id: string, guiaId: string, o: OpcoesDeEscrita): Promise<AgendamentoCriado> =>
        criar<AgendamentoCriado>('POST', `/v1/agendamentos/${seg(id)}/guia`, { guia_id: guiaId }, o),
    },

    laudos: {
      fila: (): Promise<ItemDaFilaDeLaudos[]> => ler<ItemDaFilaDeLaudos[]>('/v1/laudos/fila'),
      obter: (id: string): Promise<Laudo> => ler<Laudo>(`/v1/laudos/${seg(id)}`),
      /** PDF da versão assinada vigente (409 se ainda não assinado); a leitura é auditada. */
      baixarPdf: (id: string): Promise<DocumentoBaixado> => baixar({ metodo: 'GET', caminho: `/v1/laudos/${seg(id)}/pdf` }),
      /** Salvamento automático do rascunho (R6). Repetir grava o mesmo texto. */
      salvarRascunho: (id: string, corpo: PedidoRascunho): Promise<Laudo> =>
        escrever<Laudo>('PUT', `/v1/laudos/${seg(id)}/rascunho`, corpo),
      assinar: (id: string, corpo: PedidoAssinatura, o: OpcoesDeEscrita): Promise<Laudo> =>
        criar<Laudo>('POST', `/v1/laudos/${seg(id)}/assinatura`, corpo, o),
      retificar: (id: string, corpo: PedidoRetificacao, o: OpcoesDeEscrita): Promise<Laudo> =>
        criar<Laudo>('POST', `/v1/laudos/${seg(id)}/retificacao`, corpo, o),
      conversarAntes: (id: string, corpo: PedidoConversarAntes): Promise<Laudo> =>
        escrever<Laudo>('POST', `/v1/laudos/${seg(id)}/conversar-antes`, corpo),
      liberar: (id: string, o: OpcoesDeEscrita): Promise<Laudo> =>
        criar<Laudo>('POST', `/v1/laudos/${seg(id)}/liberacao`, {}, o),
      /** Publicação manual no portal (409 para rascunho). */
      publicar: (id: string, o: OpcoesDeEscrita): Promise<unknown> =>
        criar<unknown>('POST', `/v1/laudos/${seg(id)}/publicacao`, {}, o),
      /** Link para médico externo; o token vem uma única vez. */
      criarLink: (id: string, horas: number, o: OpcoesDeEscrita): Promise<LinkCriado> =>
        criar<LinkCriado>('POST', `/v1/laudos/${seg(id)}/links`, { horas }, o),
      acessoDeEmergencia: (id: string, corpo: PedidoAcessoEmergencia, o: OpcoesDeEscrita): Promise<AcessoDeEmergencia> =>
        criar<AcessoDeEmergencia>('POST', `/v1/laudos/${seg(id)}/acesso-emergencia`, corpo, o),
      /** Achados estruturados da versão em edição (ou da vigente, se assinado). Leitura auditada. */
      achados: (id: string): Promise<AchadosDoLaudo> => ler<AchadosDoLaudo>(`/v1/laudos/${seg(id)}/achados`),
      /** Novo achado no rascunho: o servidor calcula categoria e conduta e grava a versão da regra. */
      criarAchado: (id: string, corpo: NovoAchado, o: OpcoesDeEscrita): Promise<AchadoEstruturado> =>
        criar<AchadoEstruturado>('POST', `/v1/laudos/${seg(id)}/achados`, corpo, o),
      /** Remove do rascunho (nada é apagado); repetir devolve a mesma lista. */
      removerAchado: (id: string, achadoId: string): Promise<AchadosDoLaudo> =>
        escrever<AchadosDoLaudo>('POST', `/v1/laudos/${seg(id)}/achados/${seg(achadoId)}/remocao`, {}),
    },

    estudos: {
      pendentes: (): Promise<EstudoPendente[]> => ler<EstudoPendente[]>('/v1/estudos/pendentes'),
      /** Autoriza e registra a abertura das imagens; `url` nula se o visualizador não estiver instalado. */
      visualizacao: (id: string): Promise<{ estudo_codigo: string; study_uid: string; imagens: number; url?: string | null }> =>
        ler<{ estudo_codigo: string; study_uid: string; imagens: number; url?: string | null }>(`/v1/estudos/${seg(id)}/visualizacao`),
      /** Conciliação manual com os dois identificadores (médico). */
      conciliar: (id: string, corpo: PedidoConciliacao, o: OpcoesDeEscrita): Promise<unknown> =>
        criar<unknown>('POST', `/v1/estudos/${seg(id)}/conciliacao`, corpo, o),
    },

    portal: {
      laudos: (): Promise<ItemDoPortal[]> => ler<ItemDoPortal[]>('/v1/portal/laudos'),
      laudo: (id: string): Promise<LaudoDoPortal> => ler<LaudoDoPortal>(`/v1/portal/laudos/${seg(id)}`),
      /** PDF do laudo publicado do próprio paciente (sessão do portal). */
      baixarPdf: (id: string): Promise<DocumentoBaixado> => baixar({ metodo: 'GET', caminho: `/v1/portal/laudos/${seg(id)}/pdf` }),
      /** Laudo compartilhado por link; `410 link-expirado` quando venceu. */
      link: (token: string): Promise<LaudoDoPortal> => ler<LaudoDoPortal>(`/v1/links/${seg(token)}`),
      /** Sessão do paciente (401 sem sessão). */
      sessao: (): Promise<SessaoDoPortal> => ler<SessaoDoPortal>('/v1/portal/sessao'),
      /** Login; erro único `401 login-nao-confere`; `429 acesso-em-espera` depois de 5 erros. */
      entrar: (corpo: EntradaNoPortal): Promise<void> => noPortal('/v1/portal/sessao', corpo),
      sair: (): Promise<void> => noPortal('/v1/portal/sessao/encerramento', {}),
      situacaoDoConvite: (token: string): Promise<SituacaoDoConvite> => ler<SituacaoDoConvite>(`/v1/portal/convites/${seg(token)}`),
      /** Aceite: confere o nascimento, grava login e senha e já entra (cookie de sessão do portal). */
      aceitarConvite: (token: string, corpo: AceiteDoConvite): Promise<void> =>
        noPortal(`/v1/portal/convites/${seg(token)}/aceite`, corpo),
    },

    caixas: {
      meu: (): Promise<Caixa | null> => ler<Caixa | null>('/v1/caixas/meu'),
      /** Atendimentos do dia para receber: paciente, horário e procedimentos pela tabela de preços. */
      aReceber: (data: string): Promise<AtendimentoAReceber[]> => ler<AtendimentoAReceber[]>(`/v1/caixa/a-receber?${new URLSearchParams({ data }).toString()}`),
      abrir: (fundoCentavos: number, o: OpcoesDeEscrita): Promise<Caixa> =>
        criar<Caixa>('POST', '/v1/caixas', { fundo_centavos: fundoCentavos }, o),
      /** Fechamento: o servidor devolve o caixa com esperado, contado e diferença. */
      fechar: (id: string, contadoCentavos: number, o: OpcoesDeEscrita): Promise<Caixa> =>
        criar<Caixa>('POST', `/v1/caixas/${seg(id)}/fechamento`, { contado_centavos: contadoCentavos }, o),
    },

    recebimentos: {
      /** O servidor calcula o total pela tabela vigente; a tela não manda total (FC-CXA-01, 02). */
      criar: (corpo: PedidoRecebimento, o: OpcoesDeEscrita): Promise<Recebimento> =>
        criar<Recebimento>('POST', '/v1/recebimentos', corpo, o),
      /** Situação atual; o PIX copia e cola aparece quando o PSP devolver a cobrança. */
      obter: (id: string): Promise<Recebimento> => ler<Recebimento>(`/v1/recebimentos/${seg(id)}`),
      /** NFS-e do recebimento; `404 nota-nao-emitida` até a prefeitura devolver. */
      nota: (id: string): Promise<NotaFiscal> => ler<NotaFiscal>(`/v1/recebimentos/${seg(id)}/nota`),
      pedirEstorno: (id: string, corpo: PedidoEstorno, o: OpcoesDeEscrita): Promise<Estorno> =>
        criar<Estorno>('POST', `/v1/recebimentos/${seg(id)}/estornos`, corpo, o),
    },

    estornos: {
      /** Por situação (padrão no servidor: os pedidos que aguardam decisão). */
      listar: (situacao: SituacaoDoEstorno = 'pedido'): Promise<EstornoDetalhado[]> =>
        ler<EstornoDetalhado[]>(`/v1/estornos?${new URLSearchParams({ situacao }).toString()}`),
      obter: (id: string): Promise<EstornoDetalhado> => ler<EstornoDetalhado>(`/v1/estornos/${seg(id)}`),
      /** Recusa com motivo (quem pediu não recusa). */
      recusar: (id: string, corpo: Decisao, o: OpcoesDeEscrita): Promise<EstornoDetalhado> =>
        criar<EstornoDetalhado>('POST', `/v1/estornos/${seg(id)}/recusa`, corpo, o),
      /** Aprovação (reautenticação; quem pediu não aprova: `403 separacao-de-funcoes`). */
      aprovar: (id: string, o: OpcoesDeEscrita): Promise<Estorno> =>
        criar<Estorno>('POST', `/v1/estornos/${seg(id)}/aprovacao`, {}, o),
    },

    menu: {
      /** Ações do objeto para o perfil e o estado atual; a tela só mostra (docs/19 § 8). */
      de: (objeto: ObjetoDoMenu, id: string): Promise<MenuDoObjeto> => ler<MenuDoObjeto>(`/v1/menu/${objeto}/${seg(id)}`),
    },

    usuarios: {
      listar: (): Promise<UsuarioAdministrado[]> => ler<UsuarioAdministrado[]>('/v1/usuarios'),
      /** Reautenticação; com perfil clínico, nasce inativo e espera a segunda pessoa. */
      criar: (corpo: NovoUsuario, o: OpcoesDeEscrita): Promise<UsuarioAdministrado> => criar<UsuarioAdministrado>('POST', '/v1/usuarios', corpo, o),
      mudar: (id: string, corpo: MudancaDePerfil, o: OpcoesDeEscrita): Promise<UsuarioAdministrado> =>
        criar<UsuarioAdministrado>('POST', `/v1/usuarios/${seg(id)}/mudancas`, corpo, o),
      decidir: (pedidoId: string, corpo: DecisaoDePerfil, o: OpcoesDeEscrita): Promise<UsuarioAdministrado> =>
        criar<UsuarioAdministrado>('POST', `/v1/usuarios/pedidos/${seg(pedidoId)}/decisao`, corpo, o),
    },

    financeiro: {
      resumo: (): Promise<ResumoFinanceiro> => ler<ResumoFinanceiro>('/v1/financeiro/resumo'),
      contas: (): Promise<ContaBancaria[]> => ler<ContaBancaria[]>('/v1/financeiro/contas'),
      titulos: (tipo: 'pagar' | 'receber', situacao = 'aberto'): Promise<Titulo[]> =>
        ler<Titulo[]>(`/v1/financeiro/titulos?${new URLSearchParams({ tipo, situacao }).toString()}`),
      lancar: (corpo: NovoTitulo, o: OpcoesDeEscrita): Promise<Titulo> => criar<Titulo>('POST', '/v1/financeiro/titulos', corpo, o),
      /** Reautenticação; quem lançou não aprova. */
      aprovar: (id: string, o: OpcoesDeEscrita): Promise<Titulo> => criar<Titulo>('POST', `/v1/financeiro/titulos/${seg(id)}/aprovacoes`, {}, o),
      baixar: (id: string, corpo: { conta_id: string; data: string; extrato_linha_id?: string | null }, o: OpcoesDeEscrita): Promise<Titulo> =>
        criar<Titulo>('POST', `/v1/financeiro/titulos/${seg(id)}/baixa`, corpo, o),
      repasse: (competencia: string): Promise<RepasseDoMedico[]> =>
        ler<RepasseDoMedico[]>(`/v1/financeiro/repasse?${new URLSearchParams({ competencia }).toString()}`),
      gerarRepasse: (competencia: string, o: OpcoesDeEscrita): Promise<RepasseGerado> =>
        criar<RepasseGerado>('POST', `/v1/financeiro/repasse/${seg(competencia)}/titulos`, {}, o),
    },

    comunicacao: {
      doPaciente: (id: string): Promise<ComunicacaoDoPaciente> => ler<ComunicacaoDoPaciente>(`/v1/pacientes/${seg(id)}/comunicacao`),
      /** Liga ou desliga o WhatsApp do paciente, com motivo. */
      preferir: (id: string, corpo: PreferenciaWhatsapp): Promise<ComunicacaoDoPaciente> =>
        escrever<ComunicacaoDoPaciente>('POST', `/v1/pacientes/${seg(id)}/comunicacao`, corpo),
      /** Confirmação por WhatsApp dos agendamentos do dia (repetir não manda de novo). */
      lembretes: (data: string): Promise<LembretesEnfileirados> => escrever<LembretesEnfileirados>('POST', '/v1/comunicacao/lembretes', { data }),
    },

    indicadores: {
      /** Agregados do período (até um ano), sem paciente. */
      doPeriodo: (de: string, ate: string): Promise<Indicadores> => ler<Indicadores>(`/v1/indicadores?${new URLSearchParams({ de, ate }).toString()}`),
    },

    lastro: {
      /** Cadeia completa a partir de qualquer código (gerência e auditor); a leitura fica no lastro. */
      cadeia: (codigo: string): Promise<CadeiaDoAtendimento> => ler<CadeiaDoAtendimento>(`/v1/lastro/cadeia?${new URLSearchParams({ codigo }).toString()}`),
      /** Dossiê em PDF da mesma cadeia; a emissão e o SHA-256 entram no lastro do atendimento. */
      dossie: (corpo: PedidoDossie): Promise<DocumentoBaixado> => baixar({ metodo: 'POST', caminho: '/v1/lastro/dossie', corpo, escrita: true }),
    },

    estoque: {
      itens: (): Promise<ItemDeEstoque[]> => ler<ItemDeEstoque[]>('/v1/estoque/itens'),
      entrada: (corpo: EntradaDeEstoque, o: OpcoesDeEscrita): Promise<MovimentoFeito> => criar<MovimentoFeito>('POST', '/v1/estoque/entradas', corpo, o),
      /** Pacientes que receberam o lote (gerência); a leitura vai ao lastro. */
      rastreio: (loteId: string): Promise<RastreioDoLote> => ler<RastreioDoLote>(`/v1/estoque/lotes/${seg(loteId)}/rastreio`),
      /** Contraste aplicado: baixa do lote que vence antes, ligada ao paciente (técnico lotado). */
      registrarContraste: (atendimentoId: string, corpo: ContrasteAplicado, o: OpcoesDeEscrita): Promise<MovimentoFeito> =>
        criar<MovimentoFeito>('POST', `/v1/atendimentos/${seg(atendimentoId)}/contraste`, corpo, o),
    },

    autorizacoes: {
      /** Pede a autorização do agendamento; o servidor monta a solicitação e escolhe o canal. */
      pedir: (corpo: NovoPedidoAutorizacao, o: OpcoesDeEscrita): Promise<AutorizacaoDetalhe> =>
        criar<AutorizacaoDetalhe>('POST', '/v1/autorizacoes', corpo, o),
      listar: (situacao?: SituacaoDaAutorizacao): Promise<AutorizacaoResumo[]> =>
        ler<AutorizacaoResumo[]>(`/v1/autorizacoes${situacao === undefined ? '' : `?${new URLSearchParams({ situacao }).toString()}`}`),
      obter: (id: string): Promise<AutorizacaoDetalhe> => ler<AutorizacaoDetalhe>(`/v1/autorizacoes/${seg(id)}`),
      /** Pedido feito no portal da operadora: protocolo. */
      registrarProtocolo: (id: string, corpo: PedidoProtocolo): Promise<AutorizacaoDetalhe> =>
        escrever<AutorizacaoDetalhe>('POST', `/v1/autorizacoes/${seg(id)}/protocolo`, corpo),
      /** Resposta copiada do portal (gravada como recebida, com hash). */
      registrarResposta: (id: string, corpo: RespostaDoPortal, o: OpcoesDeEscrita): Promise<AutorizacaoDetalhe> =>
        criar<AutorizacaoDetalhe>('POST', `/v1/autorizacoes/${seg(id)}/resposta`, corpo, o),
      consultarAgora: (id: string): Promise<AutorizacaoDetalhe> => escrever<AutorizacaoDetalhe>('POST', `/v1/autorizacoes/${seg(id)}/consulta`, {}),
    },

    faturamento: {
      /** Apura as guias dos atendimentos com guia consumida e confere todas pelas regras da operadora. */
      conferir: (o: OpcoesDeEscrita): Promise<ResultadoConferencia> =>
        criar<ResultadoConferencia>('POST', '/v1/faturamento/conferencias', {}, o),
      contas: (situacao?: SituacaoDaConta): Promise<ContaResumo[]> =>
        ler<ContaResumo[]>(`/v1/faturamento/contas${situacao === undefined ? '' : `?${new URLSearchParams({ situacao }).toString()}`}`),
      conta: (id: string): Promise<ContaDetalhe> => ler<ContaDetalhe>(`/v1/faturamento/contas/${seg(id)}`),
      /** Corrige antes do lote: versão nova com motivo e conferência de novo. */
      corrigir: (id: string, corpo: PedidoCorrecaoConta, o: OpcoesDeEscrita): Promise<ContaDetalhe> =>
        criar<ContaDetalhe>('POST', `/v1/faturamento/contas/${seg(id)}/correcao`, corpo, o),
      auditoria: (): Promise<PainelAuditoria> => ler<PainelAuditoria>('/v1/faturamento/auditoria'),
      contratos: (): Promise<ContratoDeConvenio[]> => ler<ContratoDeConvenio[]>('/v1/faturamento/contratos'),
      lotes: (): Promise<LoteTiss[]> => ler<LoteTiss[]>('/v1/faturamento/lotes'),
      lote: (id: string): Promise<LoteDetalhe> => ler<LoteDetalhe>(`/v1/faturamento/lotes/${seg(id)}`),
      /** Endereço do XML do lote para baixar (a sessão vai no cookie; a resposta é no-store). */
      enderecoDoArquivo: (id: string): string => url(`/v1/faturamento/lotes/${seg(id)}/arquivo`),
      /** Gera o lote com as guias prontas da operadora (o servidor confere cada uma de novo). */
      gerarLote: (corpo: PedidoLote, o: OpcoesDeEscrita): Promise<LoteDetalhe> =>
        criar<LoteDetalhe>('POST', '/v1/faturamento/lotes', corpo, o),
      /** Pede o envio à operadora; repetir não cria outro envio. */
      enviarLote: (id: string): Promise<LoteDetalhe> => escrever<LoteDetalhe>('POST', `/v1/faturamento/lotes/${seg(id)}/envio`, {}),
      registrarDemonstrativo: (id: string, corpo: PedidoDemonstrativo, o: OpcoesDeEscrita): Promise<LoteDetalhe> =>
        criar<LoteDetalhe>('POST', `/v1/faturamento/lotes/${seg(id)}/demonstrativo`, corpo, o),
      glosas: (): Promise<Glosa[]> => ler<Glosa[]>('/v1/faturamento/glosas'),
      recorrer: (id: string, corpo: PedidoRecurso, o: OpcoesDeEscrita): Promise<Glosa> =>
        criar<Glosa>('POST', `/v1/faturamento/glosas/${seg(id)}/recurso`, corpo, o),
    },

    pedidosDeDesconto: {
      /** O caixa pede à gerência um desconto acima do limite; o total só muda depois da autorização. */
      pedir: (corpo: NovoPedidoDesconto, o: OpcoesDeEscrita): Promise<PedidoDesconto> =>
        criar<PedidoDesconto>('POST', '/v1/pedidos-desconto', corpo, o),
      /** Gerência: os da unidade; caixa: só os próprios. */
      listar: (situacao: SituacaoDoPedidoDeDesconto = 'pedido'): Promise<PedidoDesconto[]> =>
        ler<PedidoDesconto[]>(`/v1/pedidos-desconto?${new URLSearchParams({ situacao }).toString()}`),
      obter: (id: string): Promise<PedidoDesconto> => ler<PedidoDesconto>(`/v1/pedidos-desconto/${seg(id)}`),
      recusar: (id: string, corpo: Decisao, o: OpcoesDeEscrita): Promise<PedidoDesconto> =>
        criar<PedidoDesconto>('POST', `/v1/pedidos-desconto/${seg(id)}/recusa`, corpo, o),
      cancelar: (id: string, o: OpcoesDeEscrita): Promise<PedidoDesconto> =>
        criar<PedidoDesconto>('POST', `/v1/pedidos-desconto/${seg(id)}/cancelamento`, {}, o),
    },

    autorizacoesDeDesconto: {
      criar: (corpo: PedidoAutorizacaoDesconto, o: OpcoesDeEscrita): Promise<AutorizacaoDeDesconto> =>
        criar<AutorizacaoDeDesconto>('POST', '/v1/autorizacoes-desconto', corpo, o),
    },

    contingencias: {
      /** A `chave` vai no corpo (gerada na estação, docs/30 § 6) e também como `Idempotency-Key`. */
      declarar: (corpo: PedidoContingencia): Promise<Contingencia> =>
        criar<Contingencia>('POST', '/v1/contingencias', corpo, { idempotencyKey: corpo.chave }),
      aberta: (): Promise<Contingencia | null> => ler<Contingencia | null>('/v1/contingencias/aberta'),
      /** 201 com o atendimento ou 202 com a divergência aberta (nascimento do papel difere do cadastro). */
      lancarFicha: async (id: string, corpo: PedidoFicha, o: OpcoesDeEscrita): Promise<ResultadoDaFicha> => {
        const r = await pedirComSituacao({
          metodo: 'POST',
          caminho: `/v1/contingencias/${seg(id)}/fichas`,
          corpo,
          escrita: true,
          idempotencyKey: o.idempotencyKey,
        });
        if (r.status === 202) {
          return { situacao: 'divergencia-aberta', divergencia: (r.corpo as { divergencia: Divergencia }).divergencia };
        }
        return { situacao: 'lancada', atendimento: r.corpo as AtendimentoFase1 };
      },
      lancarRecibo: (id: string, corpo: PedidoRecibo, o: OpcoesDeEscrita): Promise<ReciboLancado> =>
        criar<ReciboLancado>('POST', `/v1/contingencias/${seg(id)}/recibos`, corpo, o),
      /** Números da faixa devolvidos sem uso ou anulados. */
      numeros: (id: string, corpo: PedidoNumeros, o: OpcoesDeEscrita): Promise<unknown> =>
        criar<unknown>('POST', `/v1/contingencias/${seg(id)}/numeros`, corpo, o),
      encerrar: (id: string, o: OpcoesDeEscrita): Promise<Contingencia> =>
        criar<Contingencia>('POST', `/v1/contingencias/${seg(id)}/encerramento`, {}, o),
    },

    divergencias: {
      /** Resolução de divergência (gerência). */
      resolver: (id: string, resolucao: string, o: OpcoesDeEscrita): Promise<unknown> =>
        criar<unknown>('POST', `/v1/divergencias/${seg(id)}/resolucao`, { resolucao }, o),
    },

    /** Endereço do fluxo SSE (`abrirEventos`). */
    urlDeEventos: (): string => url('/v1/eventos'),
  };
}
