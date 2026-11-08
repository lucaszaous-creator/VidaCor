-- 0061 · Bloqueios da agenda, lista de espera e oferta de vaga (docs/04 Agenda e Lista-Espera,
-- docs/27 FC-AGD-08, DEC-2026-11-08-82 a 85). Revisão humana obrigatória antes de rodar fora de dev (G6).
--
-- Tudo que ocupa um horário de sala (agendamento ativo, bloqueio ativo, oferta aguardando resposta)
-- trava a linha da sala (SELECT ... FOR UPDATE) num gatilho antes de gravar e confere os outros dois
-- (G2, item 3). Somam-se as constraints: exclusão de bloqueios sobrepostos, uma oferta por horário e
-- uma por pedido de espera, um pedido ativo por paciente e exame. Nada é apagado: cancelar, retirar,
-- recusar e vencer são situações novas com motivo.

-- Exclusão com igualdade de uuid e sobreposição de período no mesmo índice GiST.
CREATE EXTENSION IF NOT EXISTS btree_gist;

-- Modalidade do procedimento: a vaga de uma sala só é oferecida a quem espera exame que a sala faz.
-- Nulo = qualquer sala (a confirmar com a clínica, DEC-2026-11-08-84).
ALTER TABLE agenda.procedimento ADD COLUMN modalidade text CHECK (modalidade ~ '^[A-Z]{2,4}$');
UPDATE agenda.procedimento SET modalidade = 'US' WHERE tuss IN ('40901114','40901300','40901220','40901203','40901122');
UPDATE agenda.procedimento SET modalidade = 'DX' WHERE tuss = '40801012';
UPDATE agenda.procedimento SET modalidade = 'CT' WHERE tuss IN ('41001109','41001010');

-- Cancelar exige motivo a partir de agora (linhas antigas não são reescritas: NOT VALID).
ALTER TABLE agenda.agendamento
  ADD CONSTRAINT agendamento_cancelado_tem_motivo CHECK (situacao <> 'cancelado' OR motivo IS NOT NULL) NOT VALID;

-- ---------------------------------------------------------------------------------------------
-- Bloqueio de sala e horário (feriado, manutenção, outro), sempre com motivo.
CREATE TABLE agenda.bloqueio (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  sala_id uuid NOT NULL REFERENCES agenda.sala(id),
  tipo text NOT NULL CHECK (tipo IN ('feriado','manutencao','outro')),
  inicio timestamptz NOT NULL,
  fim timestamptz NOT NULL,
  motivo text NOT NULL CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  situacao text NOT NULL DEFAULT 'ativo' CHECK (situacao IN ('ativo','retirado')),
  retirado_em timestamptz,
  retirado_por uuid REFERENCES identidade.usuario(id),
  motivo_retirada text CHECK (length(trim(motivo_retirada)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK (fim > inicio AND fim - inicio <= interval '62 days'),
  CHECK ((situacao = 'retirado') = (retirado_em IS NOT NULL AND motivo_retirada IS NOT NULL)),
  -- Dois bloqueios ativos não se sobrepõem na mesma sala (clique duplo, duas pessoas).
  CONSTRAINT bloqueio_sem_sobreposicao EXCLUDE USING gist (sala_id WITH =, tstzrange(inicio, fim) WITH &&)
    WHERE (situacao = 'ativo')
);
CREATE INDEX bloqueio_periodo_idx ON agenda.bloqueio (unidade_id, inicio);

-- ---------------------------------------------------------------------------------------------
-- Pedido de vaga na lista de espera, com prioridade clínica e preferência de horário.
CREATE TABLE agenda.espera (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  exame_tuss text NOT NULL REFERENCES agenda.procedimento(tuss),
  operadora text CHECK (length(trim(operadora)) BETWEEN 2 AND 80),
  guia_id uuid REFERENCES convenio.guia(id),
  prioridade text NOT NULL CHECK (prioridade IN ('urgente','oncologico','gestante','rotina')),
  justificativa text CHECK (length(trim(justificativa)) BETWEEN 5 AND 500),
  medico_solicitante text CHECK (length(trim(medico_solicitante)) BETWEEN 3 AND 120),
  preferencia text NOT NULL CHECK (preferencia IN ('qualquer','manha','tarde')),
  situacao text NOT NULL DEFAULT 'aguardando' CHECK (situacao IN ('aguardando','ofertado','encaixado','saiu')),
  agendamento_id uuid UNIQUE REFERENCES agenda.agendamento(id),
  motivo_saida text CHECK (length(trim(motivo_saida)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  -- Urgente pede justificativa e médico solicitante (Lista-Espera.dc.html).
  CHECK (prioridade <> 'urgente' OR (justificativa IS NOT NULL AND medico_solicitante IS NOT NULL)),
  CHECK ((situacao = 'encaixado') = (agendamento_id IS NOT NULL)),
  CHECK ((situacao = 'saiu') = (motivo_saida IS NOT NULL))
);
-- Um pedido ativo por paciente e exame (repetir não duplica).
CREATE UNIQUE INDEX espera_uma_ativa ON agenda.espera (paciente_id, exame_tuss)
  WHERE situacao IN ('aguardando','ofertado');
CREATE INDEX espera_fila_idx ON agenda.espera (unidade_id, situacao, criado_em);

-- ---------------------------------------------------------------------------------------------
-- Oferta de um horário livre a um pedido de espera, com prazo de resposta. Enquanto aguarda, o
-- horário fica reservado para ela.
CREATE TABLE agenda.oferta (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  espera_id uuid NOT NULL REFERENCES agenda.espera(id),
  sala_id uuid NOT NULL REFERENCES agenda.sala(id),
  inicio timestamptz NOT NULL,
  fim timestamptz NOT NULL,
  expira_em timestamptz NOT NULL,
  origem_vaga text NOT NULL CHECK (origem_vaga IN
    ('cancelamento','bloqueio_retirado','reserva_expirada','oferta_sem_aceite','manual')),
  situacao text NOT NULL DEFAULT 'aguardando' CHECK (situacao IN
    ('aguardando','aceita','recusada','expirada','cancelada')),
  respondida_em timestamptz,
  respondida_por uuid REFERENCES identidade.usuario(id),
  motivo text CHECK (length(trim(motivo)) BETWEEN 3 AND 300),
  -- Adiada até o fim da transação: o aceite marca a oferta antes de criar o agendamento, para o
  -- gatilho do agendamento já não ver o horário como reservado por ela.
  agendamento_id uuid UNIQUE REFERENCES agenda.agendamento(id) DEFERRABLE INITIALLY DEFERRED,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK (fim > inicio),
  CHECK (expira_em <= inicio),
  CHECK ((situacao = 'aceita') = (agendamento_id IS NOT NULL)),
  CHECK (situacao NOT IN ('recusada','cancelada') OR motivo IS NOT NULL)
);
-- Uma oferta aguardando por horário de sala e uma por pedido de espera (G2).
CREATE UNIQUE INDEX oferta_uma_por_horario ON agenda.oferta (sala_id, inicio) WHERE situacao = 'aguardando';
CREATE UNIQUE INDEX oferta_uma_por_espera ON agenda.oferta (espera_id) WHERE situacao = 'aguardando';
CREATE INDEX oferta_vencendo_idx ON agenda.oferta (expira_em) WHERE situacao = 'aguardando';

-- ---------------------------------------------------------------------------------------------
-- Gatilhos de ocupação do horário. Cada um trava a sala antes de conferir: quem chega depois espera
-- e enxerga o que o primeiro gravou (READ COMMITTED relê a cada comando).
CREATE FUNCTION agenda.conferir_horario_do_agendamento() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF NEW.situacao IN ('cancelado','faltou','expirada') THEN
    RETURN NEW;
  END IF;
  IF TG_OP = 'UPDATE' AND OLD.sala_id = NEW.sala_id AND OLD.inicio = NEW.inicio AND OLD.fim = NEW.fim
     AND OLD.situacao NOT IN ('cancelado','faltou','expirada') THEN
    RETURN NEW;
  END IF;
  PERFORM 1 FROM agenda.sala WHERE id = NEW.sala_id FOR UPDATE;
  IF EXISTS (SELECT 1 FROM agenda.bloqueio b
              WHERE b.sala_id = NEW.sala_id AND b.situacao = 'ativo'
                AND tstzrange(b.inicio, b.fim) && tstzrange(NEW.inicio, NEW.fim)) THEN
    RAISE EXCEPTION 'horário bloqueado nesta sala'
      USING ERRCODE = 'exclusion_violation', CONSTRAINT = 'horario_bloqueado';
  END IF;
  IF EXISTS (SELECT 1 FROM agenda.oferta o
              WHERE o.sala_id = NEW.sala_id AND o.situacao = 'aguardando' AND o.expira_em > now()
                AND tstzrange(o.inicio, o.fim) && tstzrange(NEW.inicio, NEW.fim)) THEN
    RAISE EXCEPTION 'horário reservado para uma oferta da lista de espera'
      USING ERRCODE = 'exclusion_violation', CONSTRAINT = 'horario_em_oferta';
  END IF;
  RETURN NEW;
END$$;
CREATE TRIGGER agendamento_confere_horario BEFORE INSERT OR UPDATE ON agenda.agendamento
  FOR EACH ROW EXECUTE FUNCTION agenda.conferir_horario_do_agendamento();

CREATE FUNCTION agenda.conferir_bloqueio() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF NEW.situacao <> 'ativo' THEN
    RETURN NEW;
  END IF;
  IF TG_OP = 'UPDATE' AND OLD.situacao = 'ativo' AND OLD.sala_id = NEW.sala_id
     AND OLD.inicio = NEW.inicio AND OLD.fim = NEW.fim THEN
    RETURN NEW;
  END IF;
  PERFORM 1 FROM agenda.sala WHERE id = NEW.sala_id FOR UPDATE;
  IF EXISTS (SELECT 1 FROM agenda.agendamento a
              WHERE a.sala_id = NEW.sala_id AND a.situacao NOT IN ('cancelado','faltou','expirada')
                AND tstzrange(a.inicio, a.fim) && tstzrange(NEW.inicio, NEW.fim))
     OR EXISTS (SELECT 1 FROM agenda.oferta o
              WHERE o.sala_id = NEW.sala_id AND o.situacao = 'aguardando' AND o.expira_em > now()
                AND tstzrange(o.inicio, o.fim) && tstzrange(NEW.inicio, NEW.fim)) THEN
    RAISE EXCEPTION 'há agendamento ou oferta no período do bloqueio'
      USING ERRCODE = 'exclusion_violation', CONSTRAINT = 'bloqueio_com_agendamentos';
  END IF;
  RETURN NEW;
END$$;
CREATE TRIGGER bloqueio_confere_periodo BEFORE INSERT OR UPDATE ON agenda.bloqueio
  FOR EACH ROW EXECUTE FUNCTION agenda.conferir_bloqueio();

CREATE FUNCTION agenda.conferir_oferta() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'UPDATE' OR NEW.situacao <> 'aguardando' THEN
    RETURN NEW;
  END IF;
  PERFORM 1 FROM agenda.sala WHERE id = NEW.sala_id FOR UPDATE;
  IF EXISTS (SELECT 1 FROM agenda.agendamento a
              WHERE a.sala_id = NEW.sala_id AND a.situacao NOT IN ('cancelado','faltou','expirada')
                AND tstzrange(a.inicio, a.fim) && tstzrange(NEW.inicio, NEW.fim))
     OR EXISTS (SELECT 1 FROM agenda.bloqueio b
              WHERE b.sala_id = NEW.sala_id AND b.situacao = 'ativo'
                AND tstzrange(b.inicio, b.fim) && tstzrange(NEW.inicio, NEW.fim)) THEN
    RAISE EXCEPTION 'o horário não está livre'
      USING ERRCODE = 'exclusion_violation', CONSTRAINT = 'oferta_horario_ocupado';
  END IF;
  RETURN NEW;
END$$;
CREATE TRIGGER oferta_confere_horario BEFORE INSERT ON agenda.oferta
  FOR EACH ROW EXECUTE FUNCTION agenda.conferir_oferta();

-- Nada disso é apagado.
CREATE FUNCTION agenda.proibir_apagar() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'registro da agenda não é apagado: use a situação com motivo';
END$$;
CREATE TRIGGER bloqueio_nao_apaga BEFORE DELETE ON agenda.bloqueio
  FOR EACH ROW EXECUTE FUNCTION agenda.proibir_apagar();
CREATE TRIGGER espera_nao_apaga BEFORE DELETE ON agenda.espera
  FOR EACH ROW EXECUTE FUNCTION agenda.proibir_apagar();
CREATE TRIGGER oferta_nao_apaga BEFORE DELETE ON agenda.oferta
  FOR EACH ROW EXECUTE FUNCTION agenda.proibir_apagar();

-- ---------------------------------------------------------------------------------------------
-- RLS forçado. Bloqueio não tem dado de paciente: quem trabalha com a agenda lê; recepção e gerência
-- gravam. Espera e oferta têm: só recepção, gerência e o núcleo.
ALTER TABLE agenda.bloqueio ENABLE ROW LEVEL SECURITY;
ALTER TABLE agenda.bloqueio FORCE ROW LEVEL SECURITY;
CREATE POLICY bloqueio_unidade ON agenda.bloqueio
  USING (unidade_id = comum.ctx_unidade()
         AND comum.ctx_tem_perfil('recepcao','gerente','tecnico','medico','faturista','sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'));

ALTER TABLE agenda.espera ENABLE ROW LEVEL SECURITY;
ALTER TABLE agenda.espera FORCE ROW LEVEL SECURITY;
CREATE POLICY espera_unidade ON agenda.espera
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'));

ALTER TABLE agenda.oferta ENABLE ROW LEVEL SECURITY;
ALTER TABLE agenda.oferta FORCE ROW LEVEL SECURITY;
CREATE POLICY oferta_unidade ON agenda.oferta
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'));

GRANT SELECT, INSERT, UPDATE ON agenda.bloqueio, agenda.espera, agenda.oferta TO vidacor_app;

-- Aviso da oferta pelo WhatsApp (só primeiro nome, dia e hora; nada de exame).
ALTER TABLE comunicacao.mensagem DROP CONSTRAINT mensagem_modelo_check;
ALTER TABLE comunicacao.mensagem ADD CONSTRAINT mensagem_modelo_check
  CHECK (modelo IN ('confirmacao','laudo_pronto','reserva_expirada','oferta_vaga'));
