-- 0080 · Minha agenda (Agenda-Medico.dc.html; DEC-2026-11-09-140 a 143): o médico pede o bloqueio da
-- própria agenda (congresso, férias, licença, compromisso) e a coordenação (gerência) aprova ou recusa.
-- Enquanto o pedido não é aprovado, a agenda continua aberta. Aprovado, nenhum agendamento novo com
-- aquele médico entra no período (gatilho com trava no banco, G2). Revisão humana obrigatória antes de
-- rodar fora de dev (G6). Só acrescenta: uma tabela, funções e um gatilho novo; nada é apagado.

CREATE TABLE agenda.ausencia_medico (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  medico_id uuid NOT NULL REFERENCES identidade.usuario(id),
  inicio timestamptz NOT NULL,
  fim timestamptz NOT NULL,
  motivo text NOT NULL CHECK (motivo IN ('congresso','ferias','licenca','pessoal')),
  detalhe text CHECK (length(trim(detalhe)) BETWEEN 3 AND 300),
  -- Quantos agendamentos ativos do médico havia no período quando ele pediu (calculado pelo núcleo).
  afetados_no_pedido int NOT NULL CHECK (afetados_no_pedido >= 0),
  situacao text NOT NULL DEFAULT 'aguardando' CHECK (situacao IN ('aguardando','aprovada','recusada','cancelada')),
  decidido_por uuid REFERENCES identidade.usuario(id),
  decidido_em timestamptz,
  motivo_decisao text CHECK (length(trim(motivo_decisao)) BETWEEN 5 AND 300),
  -- Recontado na aprovação (pode ter mudado desde o pedido).
  afetados_na_decisao int CHECK (afetados_na_decisao >= 0),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK (fim > inicio AND fim - inicio <= interval '62 days'),
  -- Só o próprio médico pede o bloqueio da sua agenda.
  CHECK (criado_por = medico_id),
  -- Quem decide nunca é o próprio médico (segunda pessoa, docs/18 separação de funções).
  CHECK (decidido_por IS NULL OR decidido_por <> medico_id),
  CHECK ((situacao IN ('aprovada','recusada')) = (decidido_por IS NOT NULL AND decidido_em IS NOT NULL)),
  CHECK (situacao <> 'recusada' OR motivo_decisao IS NOT NULL),
  -- Dois pedidos vivos do mesmo médico não se sobrepõem (clique duplo, duas abas).
  CONSTRAINT ausencia_sem_sobreposicao EXCLUDE USING gist (medico_id WITH =, tstzrange(inicio, fim) WITH &&)
    WHERE (situacao IN ('aguardando','aprovada'))
);
CREATE INDEX ausencia_medico_idx ON agenda.ausencia_medico (medico_id, inicio);
CREATE INDEX ausencia_situacao_idx ON agenda.ausencia_medico (unidade_id, situacao);

-- Nada é apagado; a decisão acontece uma vez, sobre um pedido aguardando.
CREATE FUNCTION agenda.ausencia_so_decide() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'pedido de ausência não é apagado: use a situação com motivo';
  END IF;
  IF OLD.situacao <> 'aguardando'
     OR (to_jsonb(NEW) - 'situacao' - 'decidido_por' - 'decidido_em' - 'motivo_decisao' - 'afetados_na_decisao' - 'versao')
        IS DISTINCT FROM
        (to_jsonb(OLD) - 'situacao' - 'decidido_por' - 'decidido_em' - 'motivo_decisao' - 'afetados_na_decisao' - 'versao') THEN
    RAISE EXCEPTION 'pedido de ausência já decidido ou campo que não muda';
  END IF;
  RETURN NEW;
END$$;
CREATE TRIGGER ausencia_imutavel BEFORE UPDATE OR DELETE ON agenda.ausencia_medico
  FOR EACH ROW EXECUTE FUNCTION agenda.ausencia_so_decide();

-- Trava por médico: aprovação de ausência e agendamento com o médico passam pela mesma trava
-- transacional do PostgreSQL (G2, item 3), então um não escapa do outro.
CREATE FUNCTION agenda.travar_medico(p_medico uuid) RETURNS void LANGUAGE sql AS $$
  SELECT pg_advisory_xact_lock(hashtextextended('agenda.medico:' || p_medico::text, 0))
$$;

-- Agendamento novo (ou remarcado) com o médico dentro de uma ausência aprovada é recusado.
CREATE FUNCTION agenda.conferir_ausencia_do_medico() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF NEW.medico_id IS NULL OR NEW.situacao IN ('cancelado','faltou','expirada') THEN
    RETURN NEW;
  END IF;
  IF TG_OP = 'UPDATE' AND OLD.medico_id IS NOT DISTINCT FROM NEW.medico_id AND OLD.inicio = NEW.inicio
     AND OLD.fim = NEW.fim AND OLD.situacao NOT IN ('cancelado','faltou','expirada') THEN
    RETURN NEW;
  END IF;
  PERFORM agenda.travar_medico(NEW.medico_id);
  IF EXISTS (SELECT 1 FROM agenda.ausencia_medico x
              WHERE x.medico_id = NEW.medico_id AND x.situacao = 'aprovada'
                AND tstzrange(x.inicio, x.fim) && tstzrange(NEW.inicio, NEW.fim)) THEN
    RAISE EXCEPTION 'o médico está ausente neste horário'
      USING ERRCODE = 'exclusion_violation', CONSTRAINT = 'medico_ausente';
  END IF;
  RETURN NEW;
END$$;
CREATE TRIGGER agendamento_confere_ausencia BEFORE INSERT OR UPDATE ON agenda.agendamento
  FOR EACH ROW EXECUTE FUNCTION agenda.conferir_ausencia_do_medico();

-- Agendamentos ativos do médico no período (identificadores só; a gerência não lê a agenda clínica
-- por aqui, só a contagem para decidir).
CREATE FUNCTION agenda.afetados_pela_ausencia(p_medico uuid, p_inicio timestamptz, p_fim timestamptz)
RETURNS int LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, agenda, comum AS $$
  SELECT count(*)::int FROM agenda.agendamento a
   WHERE a.medico_id = p_medico AND a.unidade_id = comum.ctx_unidade()
     AND a.situacao NOT IN ('cancelado','faltou','expirada','concluido')
     AND tstzrange(a.inicio, a.fim) && tstzrange(p_inicio, p_fim)
$$;
REVOKE ALL ON FUNCTION agenda.afetados_pela_ausencia(uuid, timestamptz, timestamptz) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION agenda.afetados_pela_ausencia(uuid, timestamptz, timestamptz) TO vidacor_app;
GRANT EXECUTE ON FUNCTION agenda.travar_medico(uuid) TO vidacor_app;

ALTER TABLE agenda.ausencia_medico ENABLE ROW LEVEL SECURITY;
ALTER TABLE agenda.ausencia_medico FORCE ROW LEVEL SECURITY;
-- Leitura: o próprio médico; a gerência e a recepção da unidade (a recepção precisa saber quem está
-- ausente para remarcar); o núcleo. Escrita: o médico pede o seu; a gerência decide; o médico cancela
-- o próprio pedido aguardando.
CREATE POLICY ausencia_leitura ON agenda.ausencia_medico FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND (
    comum.ctx_tem_perfil('gerente','recepcao','sistema')
    OR medico_id = NULLIF(current_setting('app.usuario_id', true), '')::uuid));
CREATE POLICY ausencia_pedido ON agenda.ausencia_medico FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('medico')
    AND medico_id = NULLIF(current_setting('app.usuario_id', true), '')::uuid);
CREATE POLICY ausencia_decisao ON agenda.ausencia_medico FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND (comum.ctx_tem_perfil('gerente')
    OR medico_id = NULLIF(current_setting('app.usuario_id', true), '')::uuid))
  WITH CHECK (unidade_id = comum.ctx_unidade());
GRANT SELECT, INSERT, UPDATE ON agenda.ausencia_medico TO vidacor_app;
