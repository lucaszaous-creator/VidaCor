import { useQueryClient } from '@tanstack/react-query';
import type { AvisoAutomatico, MudancaDeAviso } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Chip, formatarDataHora } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

/** Um aviso automático: a gerência liga ou desliga com motivo, pela versão vista (409 se outra pessoa mudou). */
function LinhaDoAviso({ a, gerencia }: { a: AvisoAutomatico; gerencia: boolean }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [mudando, setMudando] = useState(false);
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((m: MudancaDeAviso, chave: string) => api.comunicacao.mudarAviso(a.modelo, m, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['comunicacao'] }),
  });
  // A resposta do servidor vale na hora (a tela nunca mente).
  const atual = op.resultado ?? a;
  return (
    <li className="col" style={{ gap: 8, padding: '14px 0', borderBottom: '1px solid var(--line)' }} aria-label={atual.nome}>
      <div className="row" style={{ gap: 12, alignItems: 'flex-start' }}>
        <span style={{ flex: 1, minWidth: 0 }}>
          <b className="t15" style={{ display: 'block' }}>
            {atual.nome}
          </b>
          <span className="t13 mute">{atual.descricao}</span>
        </span>
        <Chip cor={atual.ativo ? 'verde' : 'cinza'} texto={atual.ativo ? 'Ligado' : 'Desligado'} />
        {gerencia && !mudando && op.resultado === null && (
          <Botao
            pequeno
            onPress={() => {
              setMudando(true);
              setTentou(false);
              setMotivo('');
            }}
          >
            {atual.ativo ? 'Desligar' : 'Ligar'}
          </Botao>
        )}
      </div>
      {atual.motivo !== null && atual.motivo !== undefined && (
        <span className="t13 mute">
          {atual.ativo ? 'Ligado' : 'Desligado'} por {atual.alterado_por ?? 'alguém da gerência'}
          {atual.alterado_em === null || atual.alterado_em === undefined ? '' : ` em ${formatarDataHora(atual.alterado_em, FUSO_DA_CLINICA)}`}: {atual.motivo}
        </span>
      )}
      {op.resultado !== null && (
        <Aviso nivel="feito" titulo={op.resultado.ativo ? 'Aviso ligado.' : 'Aviso desligado.'}>
          {op.resultado.ativo ? 'Os próximos avisos voltam a sair.' : 'Os próximos avisos não saem e ficam registrados como desligados.'} A mudança ficou no lastro.
        </Aviso>
      )}
      {mudando && op.resultado === null && (
        <form
          className="col"
          style={{ gap: 8 }}
          aria-label={`${a.ativo ? 'Desligar' : 'Ligar'} ${a.nome}`}
          noValidate
          onSubmit={(ev) => {
            ev.preventDefault();
            setTentou(true);
            if (motivo.trim().length >= 5) op.enviar({ ativo: !a.ativo, motivo: motivo.trim(), versao_vista: a.versao_id ?? null });
          }}
        >
          <Campo rotulo="Motivo" value={motivo} onChange={setMotivo} erro={tentou && motivo.trim().length < 5 ? 'Escreva o motivo (5 letras ou mais).' : null} />
          {a.ativo && (
            <p className="t13 mute" style={{ margin: 0 }}>
              Desligado, o aviso não sai para nenhum paciente desta unidade até ser ligado de novo.
            </p>
          )}
          {op.erro !== null && <ErroDoAto erro={op.erro} ato="mudar o aviso" />}
          <div className="row" style={{ gap: 8 }}>
            <Botao
              pequeno
              onPress={() => {
                setMudando(false);
              }}
            >
              Voltar
            </Botao>
            <Botao pequeno type="submit" variante="principal" ocupado={op.enviando}>
              {a.ativo ? 'Desligar aviso' : 'Ligar aviso'}
            </Botao>
          </div>
        </form>
      )}
    </li>
  );
}

export function AvisosAutomaticos({ avisos, gerencia }: { avisos: AvisoAutomatico[]; gerencia: boolean }) {
  return (
    <ul className="col" style={{ gap: 0, listStyle: 'none', margin: 0, padding: 0 }} aria-label="Mensagens automáticas">
      {avisos.map((a) => (
        <LinhaDoAviso key={a.modelo} a={a} gerencia={gerencia} />
      ))}
    </ul>
  );
}
