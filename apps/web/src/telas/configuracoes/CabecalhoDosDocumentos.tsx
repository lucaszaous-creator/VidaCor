import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { CabecalhoDaUnidade, NovoCabecalho } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Escolha, formatarDataHora, ListaDeDados } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

const CHAVE = ['cabecalho-dos-documentos'];
const UFS = ['AC', 'AL', 'AP', 'AM', 'BA', 'CE', 'DF', 'ES', 'GO', 'MA', 'MT', 'MS', 'MG', 'PA', 'PB', 'PR', 'PE', 'PI', 'RJ', 'RN', 'RS', 'RO', 'RR', 'SC', 'SP', 'SE', 'TO'];

function Formulario({ vigente }: { vigente: CabecalhoDaUnidade | null }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const crm = /^CRM-([A-Z]{2}) (\d+)$/.exec(vigente?.responsavel_crm ?? '');
  const [nome, setNome] = useState(vigente?.nome ?? '');
  const [razao, setRazao] = useState(vigente?.razao_social ?? '');
  const [cnpj, setCnpj] = useState(vigente?.cnpj ?? '');
  const [endereco, setEndereco] = useState(vigente?.endereco ?? '');
  const [telefone, setTelefone] = useState(vigente?.telefone ?? '');
  const [site, setSite] = useState(vigente?.site ?? '');
  const [responsavel, setResponsavel] = useState(vigente?.responsavel_nome ?? '');
  const [uf, setUf] = useState(crm?.[1] ?? '');
  const [numero, setNumero] = useState(crm?.[2] ?? '');
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovoCabecalho, chave: string) => api.cabecalho.publicar(n, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: CHAVE }),
  });
  const primeira = vigente === null;
  const cnpjOk = cnpj.replace(/\D/g, '').length === 14;
  const ok =
    nome.trim().length >= 2 &&
    razao.trim().length >= 2 &&
    cnpjOk &&
    endereco.trim().length >= 5 &&
    telefone.trim().length >= 8 &&
    responsavel.trim().length >= 5 &&
    uf !== '' &&
    /^\d{1,10}$/.test(numero.trim()) &&
    (primeira || motivo.trim().length >= 5);
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Cabeçalho dos documentos"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (!ok) return;
        op.enviar({
          numero_atual: vigente?.numero ?? null,
          nome: nome.trim(),
          razao_social: razao.trim(),
          cnpj: cnpj.trim(),
          endereco: endereco.trim(),
          telefone: telefone.trim(),
          site: site.trim() === '' ? null : site.trim(),
          responsavel_nome: responsavel.trim(),
          responsavel_crm_uf: uf,
          responsavel_crm_numero: numero.trim(),
          motivo: motivo.trim() === '' ? null : motivo.trim(),
        });
      }}
    >
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)',
          gap: 10,
          alignItems: 'start',
        }}
      >
        <Campo rotulo="Nome fantasia" value={nome} onChange={setNome} erro={tentou && nome.trim().length < 2 ? 'Diga o nome impresso.' : null} />
        <Campo rotulo="CNPJ" value={cnpj} onChange={setCnpj} inputMode="numeric" ajuda="00.000.000/0000-00" erro={tentou && !cnpjOk ? 'O CNPJ tem 14 algarismos.' : null} />
      </div>
      <Campo rotulo="Razão social" value={razao} onChange={setRazao} erro={tentou && razao.trim().length < 2 ? 'Diga a razão social.' : null} />
      <Campo rotulo="Endereço" value={endereco} onChange={setEndereco} erro={tentou && endereco.trim().length < 5 ? 'Diga o endereço.' : null} />
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)',
          gap: 10,
          alignItems: 'start',
        }}
      >
        <Campo rotulo="Telefone" value={telefone} onChange={setTelefone} inputMode="tel" erro={tentou && telefone.trim().length < 8 ? 'Diga o telefone.' : null} />
        <Campo rotulo="Site (opcional)" value={site} onChange={setSite} ajuda="Sem http, ex.: vidacorimagem.com.br" />
      </div>
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1.6fr) 90px minmax(0, 1fr)',
          gap: 10,
          alignItems: 'start',
        }}
      >
        <Campo rotulo="Responsável técnico" value={responsavel} onChange={setResponsavel} erro={tentou && responsavel.trim().length < 5 ? 'Nome completo.' : null} />
        <Escolha rotulo="UF do CRM" valor={uf} aoMudar={setUf} vazio="UF" opcoes={UFS.map((u) => ({ valor: u, rotulo: u }))} erro={tentou && uf === '' ? 'UF.' : null} />
        <Campo rotulo="CRM" value={numero} onChange={setNumero} inputMode="numeric" erro={tentou && !/^\d{1,10}$/.test(numero.trim()) ? 'Só algarismos.' : null} />
      </div>
      {!primeira && <Campo rotulo="Motivo da mudança" value={motivo} onChange={setMotivo} erro={tentou && motivo.trim().length < 5 ? 'Diga o motivo (5 letras ou mais).' : null} />}
      <div aria-live="polite">
        {op.resultado !== null && op.erro === null && (
          <Aviso nivel="feito" titulo={`Cabeçalho versão ${String(op.resultado.vigente?.numero ?? '')} publicado.`}>
            Vale para os próximos documentos; os já emitidos continuam como saíram. Registrado no lastro.
          </Aviso>
        )}
        {op.erro !== null && <ErroDoAto erro={op.erro} ato="publicar o cabeçalho" />}
      </div>
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Publicar versão nova
        </Botao>
      </div>
    </form>
  );
}

/**
 * Cabeçalho impresso no laudo e no dossiê em PDF (gerência). Cada mudança é uma versão nova com motivo;
 * a anterior fica guardada. Sem cadastro, o PDF sai só com "VidaCor Imagem" e o nome da unidade.
 */
export function CabecalhoDosDocumentos() {
  const { api } = useDependencias();
  const c = useQuery({ queryKey: CHAVE, queryFn: () => api.cabecalho.ver() });
  const vigente = c.data?.vigente ?? null;
  return (
    <div className="col" style={{ gap: 12 }}>
      <h2 className="t15 b" style={{ margin: 0 }}>
        Cabeçalho dos documentos
      </h2>
      <EstadoDaConsulta
        oQue="o cabeçalho"
        carregando={c.isPending}
        erro={c.error}
        aoTentarDeNovo={() => void c.refetch()}
        semPermissao={{
          titulo: 'O cabeçalho é da gerência',
          detalhe: 'Peça à gerência para mudar o que sai impresso nos documentos.',
        }}
      />
      {c.data !== undefined && (
        <>
          {vigente === null ? (
            <Aviso nivel="atencao" titulo="Cabeçalho não cadastrado.">
              Os documentos saem só com "VidaCor Imagem" e o nome da unidade até a primeira versão.
            </Aviso>
          ) : (
            <ListaDeDados
              larguraDoRotulo={150}
              itens={[
                {
                  rotulo: 'Versão vigente',
                  valor: `${String(vigente.numero)}, desde ${formatarDataHora(vigente.desde, FUSO_DA_CLINICA)}${typeof vigente.publicado_por === 'string' ? `, por ${vigente.publicado_por}` : ''}`,
                },
                {
                  rotulo: 'Impresso no rodapé',
                  valor: vigente.responsavel_tecnico ?? 'Sem responsável técnico',
                },
              ]}
            />
          )}
          <Formulario vigente={vigente} />
          {c.data.versoes.length > 1 && (
            <>
              <h3 className="t14 b" style={{ margin: 0 }}>
                Versões anteriores
              </h3>
              <ul className="col" style={{ gap: 6, listStyle: 'none', margin: 0, padding: 0 }}>
                {c.data.versoes
                  .filter((v) => v.numero !== vigente?.numero)
                  .map((v) => (
                    <li
                      key={v.numero}
                      className="t13"
                      style={{
                        borderBottom: '1px solid var(--line)',
                        paddingBottom: 6,
                      }}
                    >
                      <b>Versão {v.numero}</b>, {formatarDataHora(v.desde, FUSO_DA_CLINICA)}
                      {typeof v.publicado_por === 'string' ? `, por ${v.publicado_por}` : ''}
                      {typeof v.motivo === 'string' ? `: ${v.motivo}` : ''}.
                    </li>
                  ))}
              </ul>
            </>
          )}
        </>
      )}
    </div>
  );
}
