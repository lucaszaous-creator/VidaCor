import { useQuery } from '@tanstack/react-query';
import type { PacienteResumo } from '@vidacor/api-client';
import { Botao, Busca, formatarData, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { consultaPacientes } from '../consultas';
import { useDependencias } from '../contexto';
import { EstadoDaConsulta } from './Bloco';
import { LinhaDoPaciente } from './LinhaDoPaciente';

export interface PropsDaEscolhaDePaciente {
  /** Rótulo da busca ("Paciente da ficha"). */
  rotulo: string;
  paciente: PacienteResumo | null;
  aoEscolher: (p: PacienteResumo | null) => void;
  erro?: string | null;
}

/** Busca de paciente no cadastro (`GET /v1/pacientes?busca=`) com os dois identificadores. */
export function EscolhaDePaciente({ rotulo, paciente, aoEscolher, erro }: PropsDaEscolhaDePaciente) {
  const { api } = useDependencias();
  const [texto, setTexto] = useState('');
  const t = texto.trim();
  const busca = useQuery({ ...consultaPacientes(api, t), enabled: t.length >= 3 && paciente === null });
  if (paciente !== null) {
    return (
      <div className="row" style={{ gap: 10 }}>
        <span style={{ flex: 1, minWidth: 0 }}>
          <span className="t13 mute" style={{ display: 'block' }}>
            {rotulo}
          </span>
          <b>{paciente.nome}</b>, nascimento <b className="num">{formatarData(paciente.nascimento)}</b>
        </span>
        <Botao
          pequeno
          onPress={() => {
            aoEscolher(null);
          }}
        >
          Trocar paciente
        </Botao>
      </div>
    );
  }
  return (
    <div className="col" style={{ gap: 6 }}>
      <span className="t13 b mute">{rotulo}</span>
      <Busca rotulo={rotulo} valor={texto} aoMudar={setTexto} placeholder="Nome do paciente (mínimo 3 letras)" />
      {erro !== undefined && erro !== null && <span className="campo-erro">{erro}</span>}
      {t.length >= 3 && (
        <div>
          <EstadoDaConsulta oQue="os pacientes" carregando={busca.isPending} erro={busca.error} aoTentarDeNovo={() => void busca.refetch()} linhas={2} />
          {busca.data?.length === 0 && <Vazio titulo="Nenhum paciente com esse nome" detalhe="Confira a grafia ou busque pelo sobrenome." />}
          {busca.data?.map((p) => (
            <LinhaDoPaciente
              key={p.id}
              nome={p.nome}
              nascimento={p.nascimento}
              selecionada={false}
              aoAbrir={() => {
                aoEscolher(p);
              }}
            />
          ))}
        </div>
      )}
    </div>
  );
}
