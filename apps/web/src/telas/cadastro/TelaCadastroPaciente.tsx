import type { CalendarDate } from '@internationalized/date';
import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { ErroDaApi, type NovoPaciente, type PacienteCadastrado, type PossivelDuplicado } from '@vidacor/api-client';
import { Aviso, Botao, CaixaDeMarcar, Campo, CampoDeData, Escolha, formatarData } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

const SEXOS = [
  { valor: 'F', rotulo: 'Feminino' },
  { valor: 'M', rotulo: 'Masculino' },
];

/**
 * Cadastro de paciente (Cadastro-Paciente.dc.html). Antes de salvar, o servidor procura cadastro
 * parecido (nome e nascimento, CPF); a recepção decide se é a mesma pessoa. O mínimo para agendar é
 * nome, nascimento, sexo, celular e alergias (informadas ou negadas); o servidor confere tudo de novo.
 */
export function TelaCadastroPaciente() {
  const { api } = useDependencias();
  const [nome, setNome] = useState('');
  const [nomeSocial, setNomeSocial] = useState('');
  const [nascimento, setNascimento] = useState<CalendarDate | null>(null);
  const [sexo, setSexo] = useState('');
  const [cpf, setCpf] = useState('');
  const [mae, setMae] = useState('');
  const [celular, setCelular] = useState('');
  const [email, setEmail] = useState('');
  const [alergias, setAlergias] = useState('');
  const [nega, setNega] = useState(false);
  const [outraPessoa, setOutraPessoa] = useState(false);
  const [tentou, setTentou] = useState(false);

  const nomeCompleto = nome.trim().split(/\s+/).length >= 2;
  const dataTexto = nascimento?.toString() ?? '';
  const duplicados = useQuery({
    queryKey: ['possiveis-duplicados', nome.trim(), dataTexto, cpf.replace(/\D/g, '')],
    queryFn: () => api.pacientes.possiveisDuplicados(nome.trim(), dataTexto, cpf.replace(/\D/g, '')),
    enabled: nomeCompleto && nascimento !== null,
    staleTime: 30_000,
  });
  const op = useOperacao((p: NovoPaciente, chave: string) => api.pacientes.cadastrar(p, { idempotencyKey: chave }));
  const doServidor = op.erro instanceof ErroDaApi ? (op.erro.campo('candidatos') as PossivelDuplicado[] | undefined) : undefined;
  const parecidos = doServidor ?? duplicados.data ?? [];
  const bloqueante = parecidos.some((c) => c.bloqueante);
  const pendente = parecidos.length > 0 && (!outraPessoa || bloqueante);
  const listaDeAlergias = alergias
    .split(/[,;\n]/)
    .map((a) => a.trim())
    .filter((a) => a !== '');

  const erros = {
    nome: tentou && !nomeCompleto ? 'Digite o nome completo, com sobrenome.' : null,
    nascimento: tentou && nascimento === null ? 'Digite a data de nascimento.' : null,
    sexo: tentou && sexo === '' ? 'Escolha o sexo do cadastro.' : null,
    celular: tentou && !/^\d{10,11}$/.test(celular.replace(/\D/g, '')) ? 'Digite o celular com DDD.' : null,
    alergias: tentou && listaDeAlergias.length === 0 && !nega ? 'Informe as alergias ou marque que o paciente nega alergias.' : null,
  };

  function salvar(): void {
    setTentou(true);
    if (Object.values(erros).some((e) => e !== null) || !nomeCompleto || nascimento === null || sexo === '' || pendente) return;
    if (listaDeAlergias.length === 0 && !nega) return;
    op.enviar({
      nome: nome.trim(),
      nome_social: nomeSocial.trim() === '' ? null : nomeSocial.trim(),
      nascimento: nascimento.toString(),
      sexo: sexo as 'F' | 'M',
      cpf: cpf.trim() === '' ? null : cpf.trim(),
      nome_mae: mae.trim() === '' ? null : mae.trim(),
      celular,
      email: email.trim() === '' ? null : email.trim(),
      alergias: nega ? [] : listaDeAlergias,
      nega_alergias: nega,
      duplicado_conferido: outraPessoa,
    });
  }

  const feito: PacienteCadastrado | null = op.resultado;
  return (
    <>
      <CabecalhoDaTela titulo="Novo paciente" complemento={<span className="t14 mute">O servidor procura cadastro parecido antes de salvar</span>} />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.4fr) minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Dados do paciente">
          <form
            className="scroll col"
            style={{ gap: 12 }}
            noValidate
            onSubmit={(ev) => {
              ev.preventDefault();
              salvar();
            }}
          >
            <h2 className="t15 b" style={{ margin: 0 }}>Identificação</h2>
            <div className="grid g2" style={{ gap: 10 }}>
              <Campo rotulo="Nome completo *" value={nome} onChange={(v) => { setNome(v); setOutraPessoa(false); }} autoComplete="off" erro={erros.nome} />
              <Campo rotulo="Nome social" value={nomeSocial} onChange={setNomeSocial} ajuda="Como prefere ser chamado." />
              <CampoDeData rotulo="Data de nascimento *" valor={nascimento} aoMudar={(v) => { setNascimento(v); setOutraPessoa(false); }} erro={erros.nascimento} />
              <Escolha rotulo="Sexo *" valor={sexo} aoMudar={setSexo} vazio="Escolha" opcoes={SEXOS} erro={erros.sexo} />
              <Campo rotulo="CPF" value={cpf} onChange={setCpf} inputMode="numeric" ajuda="Opcional; o servidor confere os dígitos." />
              <Campo rotulo="Nome da mãe" value={mae} onChange={setMae} />
            </div>
            <h2 className="t15 b" style={{ margin: 0 }}>Contato</h2>
            <div className="grid g2" style={{ gap: 10 }}>
              <Campo rotulo="Celular *" value={celular} onChange={setCelular} inputMode="tel" ajuda="Com DDD." erro={erros.celular} />
              <Campo rotulo="E-mail" value={email} onChange={setEmail} inputMode="email" />
            </div>
            <h2 className="t15 b" style={{ margin: 0 }}>Segurança</h2>
            <Campo rotulo="Alergias" value={alergias} onChange={setAlergias} isDisabled={nega} ajuda="Separe por vírgula. Ex.: iodo, dipirona." erro={erros.alergias} />
            <CaixaDeMarcar rotulo="Paciente nega alergias" marcado={nega} aoMudar={setNega} />
            {op.erro !== null && <ErroDoAto erro={op.erro} ato="salvar o cadastro" aoTentarDeNovo={salvar} />}
            {feito === null && (
              <div className="row" style={{ gap: 10 }}>
                <Botao type="submit" variante="principal" ocupado={op.enviando} isDisabled={pendente}>
                  Salvar cadastro
                </Botao>
                {pendente && <span className="t13 mute">Resolva o possível duplicado antes de salvar.</span>}
              </div>
            )}
          </form>
        </section>
        <section className="card pane" aria-label="Conferência">
          <div className="scroll col" style={{ gap: 12 }}>
            {feito !== null ? (
              <Aviso
                nivel="feito"
                titulo={`${feito.nome} cadastrado.`}
                acoes={
                  <Link to="/agendamentos/novo" className="btn btn-p btn-sm">
                    Agendar exame
                  </Link>
                }
              >
                Código <span className="num">{feito.codigo}</span>, nascimento {formatarData(feito.nascimento)}. O que falta para o check-in pode ser completado
                depois.
              </Aviso>
            ) : parecidos.length > 0 ? (
              <Aviso nivel={bloqueante ? 'bloqueante' : 'atencao'} titulo={bloqueante ? 'Este CPF já tem cadastro.' : 'Possível duplicado.'}>
                <ul style={{ margin: '6px 0', paddingLeft: 18 }}>
                  {parecidos.map((c) => (
                    <li key={c.paciente.id}>
                      <b>{c.paciente.nome}</b>, nascimento <span className="num">{formatarData(c.paciente.nascimento)}</span>, {c.codigo}. {c.motivo}.
                    </li>
                  ))}
                </ul>
                {bloqueante
                  ? 'Use o cadastro existente: o CPF é único.'
                  : outraPessoa
                    ? 'Marcado como outra pessoa. O lastro guarda quem conferiu.'
                    : 'Se for a mesma pessoa, use o cadastro existente. Se não for, confirme abaixo.'}
                {!bloqueante && !outraPessoa && (
                  <div className="row" style={{ marginTop: 8 }}>
                    <Botao pequeno onPress={() => { setOutraPessoa(true); op.limparErro(); }}>
                      Não é a mesma pessoa
                    </Botao>
                  </div>
                )}
              </Aviso>
            ) : (
              <Aviso nivel="informativo" titulo="Conferência de duplicado.">
                {nomeCompleto && nascimento !== null
                  ? duplicados.isFetching
                    ? 'Procurando cadastro parecido.'
                    : 'Nenhum cadastro parecido encontrado.'
                  : 'Digite o nome completo e o nascimento: o servidor procura cadastro parecido antes de salvar.'}
              </Aviso>
            )}
          </div>
        </section>
      </div>
    </>
  );
}
