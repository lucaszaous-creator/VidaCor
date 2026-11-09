import { useQueryClient } from '@tanstack/react-query';
import type { NovoPreparo, PreparoDoExame } from '@vidacor/api-client';
import { AreaDeTexto, Botao, Campo, Escolha } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

export const CHAVE_PREPAROS = ['preparos'];

const ANTECEDENCIAS = [0, 10, 15, 20, 30, 45, 60, 90, 120].map((m) => ({
  valor: String(m),
  rotulo: m === 0 ? 'No horário' : `${String(m)} min antes`,
}));

/** Uma linha por passo; linhas vazias são ignoradas (o servidor confere de novo). */
export function passosDoTexto(texto: string): string[] {
  return texto
    .split('\n')
    .map((l) => l.replace(/\s+/g, ' ').trim())
    .filter((l) => l !== '');
}

/**
 * Versão nova do preparo (gerência). Parte do texto vigente; a partir da segunda versão o motivo é
 * obrigatório. Se outra pessoa publicou antes, o servidor recusa (409) e nada muda.
 */
export function NovaVersaoDoPreparo({ exame, aoFechar }: { exame: PreparoDoExame; aoFechar: () => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const v = exame.vigente ?? null;
  const [texto, setTexto] = useState(() => (v === null ? '' : v.passos.join('\n')));
  const [antes, setAntes] = useState(String(v?.chegar_antes_min ?? 15));
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovoPreparo, chave: string) => api.preparos.publicar(exame.tuss, n, { idempotencyKey: chave }), {
    aoConcluir: async () => {
      await cliente.invalidateQueries({ queryKey: CHAVE_PREPAROS });
      aoFechar();
    },
  });
  const passos = passosDoTexto(texto);
  const passoRuim = passos.find((p) => p.length < 3 || p.length > 300);
  const erroDosPassos =
    passos.length === 0
      ? 'Escreva pelo menos um passo.'
      : passos.length > 12
        ? `São ${String(passos.length)} passos; o máximo é 12.`
        : passoRuim !== undefined
          ? 'Cada passo tem de 3 a 300 caracteres.'
          : null;
  const precisaMotivo = v !== null;
  const erroDoMotivo = precisaMotivo && motivo.trim().length < 5 ? 'Diga por que o preparo mudou (5 letras ou mais).' : null;
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Nova versão do preparo"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (erroDosPassos === null && erroDoMotivo === null)
          op.enviar({
            versao_vista: v?.numero ?? null,
            passos,
            chegar_antes_min: Number(antes),
            motivo: motivo.trim() === '' ? null : motivo.trim(),
          });
      }}
    >
      <h2 className="t15 b" style={{ margin: 0 }}>
        {v === null ? 'Cadastrar o preparo' : `Nova versão do preparo (hoje v${String(v.numero)})`}
      </h2>
      <AreaDeTexto
        rotulo="Passos, um por linha"
        ajuda="O paciente recebe os passos numerados no comprovante. Escreva como fala com ele."
        linhas={8}
        value={texto}
        onChange={setTexto}
        erro={tentou ? erroDosPassos : null}
      />
      <Escolha rotulo="Chegar antes do horário" valor={antes} aoMudar={setAntes} opcoes={ANTECEDENCIAS} />
      {precisaMotivo && <Campo rotulo="Motivo da mudança" value={motivo} onChange={setMotivo} erro={tentou ? erroDoMotivo : null} />}
      <p className="t13 mute" style={{ margin: 0 }}>
        A versão atual fica guardada. Comprovantes já entregues continuam com o texto que imprimiram.
      </p>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="publicar o preparo" />}
      <div className="row" style={{ gap: 8 }}>
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Publicar versão
        </Botao>
        <Botao onPress={aoFechar}>Voltar</Botao>
      </div>
    </form>
  );
}
