import type { CalendarDate } from '@internationalized/date';
import type { PacienteResumo, PedidoFicha, PedidoRecibo, ReciboLancado, ResultadoDaFicha } from '@vidacor/api-client';
import { Botao, Campo, CampoDeData, lerReais } from '@vidacor/design-system';
import { useId, useState, type ChangeEvent } from 'react';
import { EscolhaDePaciente } from '../../componentes/EscolhaDePaciente';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

const NUMERO = /^\d{4}$/;
const HORA = /^([01]\d|2[0-3]):[0-5]\d$/;
const TUSS = /^\d{8}$/;

/** Data e hora do papel no fuso desta estação, em RFC 3339 (conversão de formato, sem regra). */
function instante(data: CalendarDate | null, hora: string): string | null {
  if (data === null || !HORA.test(hora.trim())) return null;
  const d = new Date(`${data.toString()}T${hora.trim()}:00`);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

/** Lê o arquivo pelo FileReader (disponível em todo navegador e no ambiente de teste). */
function lerBytes(arquivo: File): Promise<ArrayBuffer> {
  return new Promise((resolver, rejeitar) => {
    const leitor = new FileReader();
    leitor.onload = () => {
      resolver(leitor.result as ArrayBuffer);
    };
    leitor.onerror = () => {
      rejeitar(new Error('Falha ao ler o arquivo'));
    };
    leitor.readAsArrayBuffer(arquivo);
  });
}

async function sha256(arquivo: File): Promise<string> {
  const resumo = await crypto.subtle.digest('SHA-256', new Uint8Array(await lerBytes(arquivo)));
  return [...new Uint8Array(resumo)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

export type Lancamento =
  | { tipo: 'ficha'; numero: string; resultado: ResultadoDaFicha }
  | { tipo: 'recibo'; numero: string; resultado: ReciboLancado };

interface PropsDaFicha {
  contingenciaId: string;
  aoLancar: (l: Lancamento) => void;
}

/** Lançamento posterior da ficha F-ATD (FC-CTG-03 a 06): nome e nascimento como estão no papel. */
export function FormularioDeFicha({ contingenciaId, aoLancar }: PropsDaFicha) {
  const { api } = useDependencias();
  const idArquivo = useId();
  const [numero, setNumero] = useState('');
  const [paciente, setPaciente] = useState<PacienteResumo | null>(null);
  const [nome, setNome] = useState('');
  const [nascimento, setNascimento] = useState<CalendarDate | null>(null);
  const [data, setData] = useState<CalendarDate | null>(null);
  const [hora, setHora] = useState('');
  const [atendidoPor, setAtendidoPor] = useState('');
  const [anexo, setAnexo] = useState<{ nome: string; hash: string } | null>(null);
  const [erroDoAnexo, setErroDoAnexo] = useState<string | null>(null);
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((f: PedidoFicha, chave: string) => api.contingencias.lancarFicha(contingenciaId, f, { idempotencyKey: chave }), {
    aoConcluir: (r) => {
      aoLancar({ tipo: 'ficha', numero, resultado: r });
      op.reiniciar();
      setNumero('');
      setPaciente(null);
      setNome('');
      setNascimento(null);
      setHora('');
      setAnexo(null);
      setTentou(false);
    },
  });
  const ocorrido = instante(data, hora);

  async function anexar(ev: ChangeEvent<HTMLInputElement>): Promise<void> {
    const arquivo = ev.target.files?.[0];
    setAnexo(null);
    setErroDoAnexo(null);
    if (arquivo === undefined) return;
    try {
      setAnexo({ nome: arquivo.name, hash: await sha256(arquivo) });
    } catch {
      setErroDoAnexo('Não foi possível ler o arquivo. Digitalize de novo e anexe.');
    }
  }

  function lancar(): void {
    setTentou(true);
    if (!NUMERO.test(numero.trim()) || paciente === null || nome.trim() === '' || nascimento === null || ocorrido === null || atendidoPor.trim() === '' || anexo === null) {
      return;
    }
    op.enviar({
      formulario: 'F-ATD',
      numero: numero.trim(),
      paciente_id: paciente.id,
      nome: nome.trim(),
      nascimento: nascimento.toString(),
      ocorrido_em: ocorrido,
      atendido_por: atendidoPor.trim(),
      anexo_sha256: anexo.hash,
    });
  }

  return (
    <form
      className="col"
      style={{ gap: 10 }}
      noValidate
      aria-label="Ficha F-ATD"
      onSubmit={(ev) => {
        ev.preventDefault();
        lancar();
      }}
    >
      <h3 className="t15 b">Ficha de atendimento F-ATD</h3>
      <Campo rotulo="Número da ficha" value={numero} onChange={setNumero} inputMode="numeric" ajuda="Quatro dígitos, como no papel (ex.: 0412)." erro={tentou && !NUMERO.test(numero.trim()) ? 'Digite os quatro dígitos da ficha.' : null} />
      <EscolhaDePaciente rotulo="Paciente no cadastro" paciente={paciente} aoEscolher={setPaciente} erro={tentou && paciente === null ? 'Escolha o paciente no cadastro.' : null} />
      <div className="grid g2" style={{ gap: 10 }}>
        <Campo rotulo="Nome como está no papel" value={nome} onChange={setNome} erro={tentou && nome.trim() === '' ? 'Copie o nome do papel.' : null} />
        <CampoDeData rotulo="Nascimento como está no papel" valor={nascimento} aoMudar={setNascimento} erro={tentou && nascimento === null ? 'Copie o nascimento do papel.' : null} />
        <CampoDeData rotulo="Dia do atendimento" valor={data} aoMudar={setData} erro={tentou && data === null ? 'Digite o dia.' : null} />
        <Campo rotulo="Hora do atendimento" value={hora} onChange={setHora} placeholder="10:50" erro={tentou && !HORA.test(hora.trim()) ? 'Digite a hora como 10:50.' : null} />
      </div>
      <Campo rotulo="Atendido por (como no papel)" value={atendidoPor} onChange={setAtendidoPor} erro={tentou && atendidoPor.trim() === '' ? 'Digite quem atendeu.' : null} />
      <div className="field">
        <label htmlFor={idArquivo}>Ficha digitalizada (PDF)</label>
        <input id={idArquivo} type="file" accept="application/pdf" className="in" style={{ paddingTop: 10 }} onChange={(ev) => void anexar(ev)} />
        <span className="campo-ajuda">{anexo === null ? 'O hash do arquivo vai junto para o lastro.' : `${anexo.nome}, SHA-256 ${anexo.hash.slice(0, 16)}…`}</span>
        {(erroDoAnexo !== null || (tentou && anexo === null)) && <span className="campo-erro">{erroDoAnexo ?? 'Anexe a ficha digitalizada.'}</span>}
      </div>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="lançar a ficha" aoTentarDeNovo={lancar} />}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Lançar ficha
        </Botao>
      </div>
    </form>
  );
}

/** Lançamento do recibo manual F-REC (FC-CTG-07): o servidor recalcula e abre divergência de preço. */
export function FormularioDeRecibo({ contingenciaId, aoLancar }: PropsDaFicha) {
  const { api } = useDependencias();
  const [numero, setNumero] = useState('');
  const [paciente, setPaciente] = useState<PacienteResumo | null>(null);
  const [tuss, setTuss] = useState('');
  const [valor, setValor] = useState('');
  const [data, setData] = useState<CalendarDate | null>(null);
  const [hora, setHora] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((r: PedidoRecibo, chave: string) => api.contingencias.lancarRecibo(contingenciaId, r, { idempotencyKey: chave }), {
    aoConcluir: (r) => {
      aoLancar({ tipo: 'recibo', numero, resultado: r });
      op.reiniciar();
      setNumero('');
      setPaciente(null);
      setTuss('');
      setValor('');
      setTentou(false);
    },
  });
  const centavos = lerReais(valor);
  const ocorrido = instante(data, hora);

  function lancar(): void {
    setTentou(true);
    if (!NUMERO.test(numero.trim()) || paciente === null || !TUSS.test(tuss.trim()) || centavos === null || ocorrido === null) return;
    op.enviar({ formulario: 'F-REC', numero: numero.trim(), paciente_id: paciente.id, tuss: tuss.trim(), valor_papel_centavos: centavos, ocorrido_em: ocorrido });
  }

  return (
    <form
      className="col"
      style={{ gap: 10 }}
      noValidate
      aria-label="Recibo F-REC"
      onSubmit={(ev) => {
        ev.preventDefault();
        lancar();
      }}
    >
      <h3 className="t15 b">Recibo manual F-REC</h3>
      <Campo rotulo="Número do recibo" value={numero} onChange={setNumero} inputMode="numeric" erro={tentou && !NUMERO.test(numero.trim()) ? 'Digite os quatro dígitos do recibo.' : null} />
      <EscolhaDePaciente rotulo="Paciente do recibo" paciente={paciente} aoEscolher={setPaciente} erro={tentou && paciente === null ? 'Escolha o paciente no cadastro.' : null} />
      <div className="grid g2" style={{ gap: 10 }}>
        <Campo rotulo="Procedimento (TUSS)" value={tuss} onChange={setTuss} inputMode="numeric" erro={tentou && !TUSS.test(tuss.trim()) ? 'O código TUSS tem 8 dígitos.' : null} />
        <Campo rotulo="Valor escrito no papel (R$)" value={valor} onChange={setValor} inputMode="decimal" erro={tentou && centavos === null ? 'Digite o valor em reais, ex.: 300,00.' : null} />
        <CampoDeData rotulo="Dia do recibo" valor={data} aoMudar={setData} erro={tentou && data === null ? 'Digite o dia.' : null} />
        <Campo rotulo="Hora do recibo" value={hora} onChange={setHora} placeholder="11:05" erro={tentou && !HORA.test(hora.trim()) ? 'Digite a hora como 11:05.' : null} />
      </div>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="lançar o recibo" aoTentarDeNovo={lancar} />}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Lançar recibo
        </Botao>
      </div>
    </form>
  );
}
