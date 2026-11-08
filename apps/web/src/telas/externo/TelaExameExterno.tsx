import type { CalendarDate } from '@internationalized/date';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { ImportacaoExterna, PacienteResumo } from '@vidacor/api-client';
import { Aviso, Botao, CaixaDeMarcar, Campo, CampoDeData, Chip, Escolha, formatarData, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela } from '../../componentes/Bloco';
import { EscolhaDePaciente } from '../../componentes/EscolhaDePaciente';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

const ORIGENS = [
  { valor: 'link', rotulo: 'Link recebido' },
  { valor: 'arquivo', rotulo: 'Arquivos do paciente' },
  { valor: 'dicomweb', rotulo: 'DICOMweb da instituição' },
];

/**
 * Importar exame externo (Exame-Externo.dc.html, docs/22 E3). A recepção confere nome e nascimento
 * com o documento; o servidor recusa nascimento diferente e nome incompatível. Entra só para leitura
 * e comparação: nunca vira laudo da VidaCor; a identificação de origem fica guardada.
 */
export function TelaExameExterno() {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [paciente, setPaciente] = useState<PacienteResumo | null>(null);
  const [instituicao, setInstituicao] = useState('');
  const [origem, setOrigem] = useState('link');
  const [nomeOrigem, setNomeOrigem] = useState('');
  const [nascOrigem, setNascOrigem] = useState<CalendarDate | null>(null);
  const [idOrigem, setIdOrigem] = useState('');
  const [uid, setUid] = useState('');
  const [descricao, setDescricao] = useState('');
  const [modalidade, setModalidade] = useState('');
  const [dataEstudo, setDataEstudo] = useState<CalendarDate | null>(null);
  const [imagens, setImagens] = useState('');
  const [hash, setHash] = useState('');
  const [conferido, setConferido] = useState(false);
  const [tentou, setTentou] = useState(false);
  const existentes = useQuery({ queryKey: ['externos', paciente?.id], queryFn: () => api.pacientes.examesExternos(paciente?.id ?? ''), enabled: paciente !== null });
  const op = useOperacao((i: ImportacaoExterna, chave: string) => api.pacientes.importarExameExterno(paciente?.id ?? '', i, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['externos'] }),
  });
  const ok =
    paciente !== null && instituicao.trim().length >= 2 && nomeOrigem.trim() !== '' && nascOrigem !== null && /^[0-9.]{3,64}$/.test(uid.trim()) &&
    descricao.trim() !== '' && /^[A-Z]{2,4}$/.test(modalidade.trim()) && dataEstudo !== null && /^\d+$/.test(imagens) && /^[0-9a-f]{64}$/.test(hash.trim()) && conferido;

  return (
    <>
      <CabecalhoDaTela titulo="Importar exame externo" complemento={<span className="t14 mute">Só leitura; nunca vira laudo da VidaCor</span>} />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.3fr) minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Importação">
          <form
            className="scroll col"
            style={{ gap: 10 }}
            noValidate
            onSubmit={(ev) => {
              ev.preventDefault();
              setTentou(true);
              if (ok) {
                op.enviar({
                  instituicao: instituicao.trim(), origem: origem as ImportacaoExterna['origem'], nome_na_origem: nomeOrigem.trim(), nascimento_na_origem: nascOrigem.toString(),
                  id_na_origem: idOrigem.trim() === '' ? null : idOrigem.trim(), documento_conferido: conferido,
                  estudos: [{ study_uid: uid.trim(), descricao: descricao.trim(), modalidade: modalidade.trim(), data_estudo: dataEstudo.toString(), imagens: Number(imagens), sha256: hash.trim() }],
                });
              }
            }}
          >
            <EscolhaDePaciente rotulo="Paciente do cadastro" paciente={paciente} aoEscolher={setPaciente} erro={tentou && paciente === null ? 'Escolha o paciente.' : null} />
            <div className="grid g2" style={{ gap: 10 }}>
              <Campo rotulo="Instituição de origem" value={instituicao} onChange={setInstituicao} erro={tentou && instituicao.trim().length < 2 ? 'Diga de onde vem.' : null} />
              <Escolha rotulo="De onde vem o exame" valor={origem} aoMudar={setOrigem} opcoes={ORIGENS} />
              <Campo rotulo="Nome no exame recebido" value={nomeOrigem} onChange={setNomeOrigem} ajuda="Como aparece nas imagens, ex.: PRADO^LUCIA^F" erro={tentou && nomeOrigem.trim() === '' ? 'Copie o nome do exame.' : null} />
              <CampoDeData rotulo="Nascimento no exame recebido" valor={nascOrigem} aoMudar={setNascOrigem} erro={tentou && nascOrigem === null ? 'Digite o nascimento do exame.' : null} />
              <Campo rotulo="ID na instituição de origem" value={idOrigem} onChange={setIdOrigem} />
            </div>
            <h2 className="t15 b" style={{ margin: 0 }}>Estudo</h2>
            <div className="grid g2" style={{ gap: 10 }}>
              <Campo rotulo="StudyUID" value={uid} onChange={setUid} erro={tentou && !/^[0-9.]{3,64}$/.test(uid.trim()) ? 'Só números e pontos.' : null} />
              <Campo rotulo="Descrição" value={descricao} onChange={setDescricao} erro={tentou && descricao.trim() === '' ? 'Descreva o estudo.' : null} />
              <Campo rotulo="Modalidade" value={modalidade} onChange={(v) => { setModalidade(v.toUpperCase()); }} ajuda="Ex.: CT, US, MR" erro={tentou && !/^[A-Z]{2,4}$/.test(modalidade.trim()) ? 'Ex.: CT.' : null} />
              <CampoDeData rotulo="Data do estudo" valor={dataEstudo} aoMudar={setDataEstudo} erro={tentou && dataEstudo === null ? 'Digite a data.' : null} />
              <Campo rotulo="Imagens" value={imagens} onChange={setImagens} inputMode="numeric" erro={tentou && !/^\d+$/.test(imagens) ? 'Quantas imagens.' : null} />
              <Campo rotulo="SHA-256 do conjunto" value={hash} onChange={setHash} erro={tentou && !/^[0-9a-f]{64}$/.test(hash.trim()) ? '64 caracteres hexadecimais.' : null} />
            </div>
            <CaixaDeMarcar rotulo="Conferi o nome e o nascimento com o documento do paciente" marcado={conferido} aoMudar={setConferido} erro={tentou && !conferido ? 'Confira com o documento antes de importar.' : null} />
            {op.erro !== null && <ErroDoAto erro={op.erro} ato="importar o exame externo" />}
            {op.resultado !== null && (
              <Aviso nivel="feito" titulo={`${op.resultado.codigo} importado como exame externo.`}>
                Só leitura, para comparação. A identificação de origem ({op.resultado.nome_na_origem}) ficou guardada.
              </Aviso>
            )}
            <div className="row">
              <Botao type="submit" variante="principal" ocupado={op.enviando}>
                Importar
              </Botao>
            </div>
          </form>
        </section>
        <section className="card pane" aria-label="Exames externos do paciente">
          <div className="scroll col" style={{ gap: 8 }}>
            {paciente === null ? (
              <Vazio titulo="Escolha o paciente." detalhe="Os exames externos já importados aparecem aqui." />
            ) : existentes.data?.length === 0 ? (
              <Vazio titulo="Nenhum exame externo." />
            ) : (
              existentes.data?.map((e) => (
                <div key={e.id} className="li" style={{ padding: '8px 0' }}>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div className="t14 b">{e.instituicao}</div>
                    <div className="t13 mute">
                      {e.estudos.map((s) => `${s.descricao} (${s.modalidade}), ${formatarData(s.data_estudo)}`).join('; ')}
                    </div>
                  </div>
                  <Chip cor="cinza" texto="Externo, só leitura" />
                </div>
              ))
            )}
          </div>
        </section>
      </div>
    </>
  );
}
