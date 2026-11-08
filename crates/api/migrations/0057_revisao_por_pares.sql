-- 0057 · Revisão por pares (Revisao-Pares.dc.html; docs/04 Laudar e Qualidade; docs/18 médico
-- revisor; DEC-2026-11-08-101 a 103). Laudos assinados são sorteados ou escolhidos pela coordenação
-- médica para segunda leitura por outro médico, nunca o autor (CHECK e gatilho). O parecer segue a
-- escala RADPEER (1, 2a, 2b, 3a, 3b) com comentário; discrepância significativa (2b, 3a, 3b) abre
-- pendência para o autor decidir se retifica. A revisão nunca altera o laudo: a retificação continua
-- pelo fluxo do laudo, com nova versão e motivo. Nada é apagado.

-- O revisor lê o laudo por um vínculo próprio, registrado quando abre o caso e encerrado ao concluir.
ALTER TABLE cuidado.vinculo DROP CONSTRAINT vinculo_motivo_check;
ALTER TABLE cuidado.vinculo ADD CONSTRAINT vinculo_motivo_check
  CHECK (motivo IN ('agendamento','laudo_atribuido','laudo_assinado','atendimento','revisao_por_par'));

CREATE TABLE laudos.revisao (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  laudo_id uuid NOT NULL REFERENCES laudos.laudo(id),
  -- Versão assinada revisada, o autor (quem assinou), o paciente, o código do laudo e o exame vêm do
  -- próprio laudo, preenchidos pelo gatilho (quem designa não lê o laudo).
  laudo_versao int NOT NULL,
  laudo_codigo text NOT NULL,
  exame text NOT NULL,
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  autor_id uuid NOT NULL REFERENCES identidade.usuario(id),
  revisor_id uuid NOT NULL REFERENCES identidade.usuario(id),
  modo text NOT NULL CHECK (modo IN ('sorteio','selecao')),
  situacao text NOT NULL DEFAULT 'designada' CHECK (situacao IN ('designada','concluida','conflito_declarado')),
  classe text CHECK (classe IN ('1','2a','2b','3a','3b')),
  concluida_em timestamptz,
  motivo_conflito text CHECK (length(trim(motivo_conflito)) BETWEEN 5 AND 500),
  conflito_em timestamptz,
  pendencia text CHECK (pendencia IN ('aguarda_autor','retificado','mantido')),
  pendencia_motivo text CHECK (length(trim(pendencia_motivo)) BETWEEN 5 AND 1000),
  pendencia_resolvida_em timestamptz,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  FOREIGN KEY (laudo_id, laudo_versao) REFERENCES laudos.versao (laudo_id, numero),
  -- Separação de funções: quem laudou não revisa o próprio laudo.
  CONSTRAINT revisao_revisor_nao_e_autor CHECK (revisor_id <> autor_id),
  CHECK ((situacao = 'concluida') = (classe IS NOT NULL AND concluida_em IS NOT NULL)),
  CHECK ((situacao = 'conflito_declarado') = (motivo_conflito IS NOT NULL AND conflito_em IS NOT NULL)),
  CHECK ((pendencia IS NOT NULL) = (situacao = 'concluida' AND classe IN ('2b','3a','3b'))),
  CHECK ((pendencia IN ('retificado','mantido')) IS NOT TRUE OR pendencia_resolvida_em IS NOT NULL),
  CHECK (pendencia IS DISTINCT FROM 'aguarda_autor' OR pendencia_resolvida_em IS NULL),
  CHECK (pendencia IS DISTINCT FROM 'mantido' OR pendencia_motivo IS NOT NULL)
);
-- G2: um revisor designado por vez em cada laudo e uma revisão (aberta ou concluída) por versão.
CREATE UNIQUE INDEX revisao_uma_designada ON laudos.revisao (laudo_id) WHERE situacao = 'designada';
CREATE UNIQUE INDEX revisao_uma_por_versao ON laudos.revisao (laudo_id, laudo_versao) WHERE situacao IN ('designada','concluida');
CREATE INDEX revisao_revisor_idx ON laudos.revisao (revisor_id, situacao);
CREATE INDEX revisao_autor_idx ON laudos.revisao (autor_id, situacao);

-- Comentário do revisor (pode citar achado do exame): só o revisor e o autor leem. Só cresce.
CREATE TABLE laudos.revisao_parecer (
  revisao_id uuid PRIMARY KEY REFERENCES laudos.revisao(id),
  comentario text NOT NULL CHECK (length(comentario) <= 2000),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL
);

-- Gatilho da revisão (SECURITY DEFINER: lê o laudo acima do RLS só para preencher e conferir a
-- linha inserida; não devolve conteúdo a ninguém).
CREATE FUNCTION laudos.proteger_revisao() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE
  v_laudo record;
  v_autor uuid;
  v_revisor record;
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'revisão por par não se apaga' USING ERRCODE = '42501';
  END IF;
  IF TG_OP = 'INSERT' THEN
    SELECT l.id, l.codigo, l.paciente_id, l.unidade_id, l.versao_vigente, g.exame_nome INTO v_laudo
      FROM laudos.laudo l
      JOIN recepcao.atendimento t ON t.id = l.atendimento_id
      JOIN agenda.agendamento g ON g.id = t.agendamento_id
     WHERE l.id = NEW.laudo_id;
    IF v_laudo.id IS NULL OR v_laudo.unidade_id <> NEW.unidade_id THEN
      RAISE EXCEPTION 'laudo não encontrado nesta unidade' USING ERRCODE = 'VCR01';
    END IF;
    IF v_laudo.versao_vigente IS NULL THEN
      RAISE EXCEPTION 'só laudo assinado vai para revisão por par' USING ERRCODE = 'VCR02';
    END IF;
    SELECT v.assinada_por INTO v_autor FROM laudos.versao v
     WHERE v.laudo_id = NEW.laudo_id AND v.numero = v_laudo.versao_vigente AND v.situacao = 'assinada';
    IF v_autor IS NULL THEN
      RAISE EXCEPTION 'só laudo assinado vai para revisão por par' USING ERRCODE = 'VCR02';
    END IF;
    SELECT u.id, u.ativo, u.unidade_id, 'medico' = ANY (u.perfis) AS medico INTO v_revisor
      FROM identidade.usuario u WHERE u.id = NEW.revisor_id;
    IF v_revisor.id IS NULL OR NOT v_revisor.ativo OR NOT v_revisor.medico OR v_revisor.unidade_id <> NEW.unidade_id THEN
      RAISE EXCEPTION 'revisor precisa ser médico ativo da unidade' USING ERRCODE = 'VCR03';
    END IF;
    NEW.laudo_versao := v_laudo.versao_vigente;
    NEW.laudo_codigo := v_laudo.codigo;
    NEW.exame := v_laudo.exame_nome;
    NEW.paciente_id := v_laudo.paciente_id;
    NEW.autor_id := v_autor;
    IF NEW.situacao <> 'designada' OR NEW.classe IS NOT NULL OR NEW.pendencia IS NOT NULL THEN
      RAISE EXCEPTION 'revisão nasce designada' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
  END IF;
  IF (to_jsonb(NEW) - ARRAY['situacao','classe','concluida_em','motivo_conflito','conflito_em','pendencia','pendencia_motivo','pendencia_resolvida_em','versao'])
     IS DISTINCT FROM (to_jsonb(OLD) - ARRAY['situacao','classe','concluida_em','motivo_conflito','conflito_em','pendencia','pendencia_motivo','pendencia_resolvida_em','versao']) THEN
    RAISE EXCEPTION 'dados da designação da revisão são imutáveis' USING ERRCODE = '42501';
  END IF;
  IF OLD.situacao = 'designada' AND NEW.situacao IN ('concluida','conflito_declarado') THEN
    RETURN NEW;
  END IF;
  IF OLD.situacao = 'concluida' AND NEW.situacao = 'concluida'
     AND NEW.classe = OLD.classe AND NEW.concluida_em = OLD.concluida_em
     AND OLD.pendencia = 'aguarda_autor' AND NEW.pendencia IN ('retificado','mantido') THEN
    RETURN NEW;
  END IF;
  RAISE EXCEPTION 'transição de revisão não permitida' USING ERRCODE = '42501';
END $$;
CREATE TRIGGER revisao_protegida BEFORE INSERT OR UPDATE OR DELETE ON laudos.revisao
  FOR EACH ROW EXECUTE FUNCTION laudos.proteger_revisao();

CREATE FUNCTION laudos.revisao_parecer_imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'parecer de revisão é imutável' USING ERRCODE = '42501'; END $$;
CREATE TRIGGER revisao_parecer_sem_alteracao BEFORE UPDATE OR DELETE ON laudos.revisao_parecer
  FOR EACH ROW EXECUTE FUNCTION laudos.revisao_parecer_imutavel();

-- RLS. Metadados da revisão (sem texto clínico): o revisor e o autor veem as suas; a coordenação
-- (perfil gerente; o núcleo exige também o perfil médico para a coordenação médica) vê as da
-- unidade e designa. O revisor conclui ou declara conflito; o autor resolve a pendência.
ALTER TABLE laudos.revisao ENABLE ROW LEVEL SECURITY;
ALTER TABLE laudos.revisao FORCE ROW LEVEL SECURITY;
CREATE POLICY revisao_leitura ON laudos.revisao FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND (
    comum.ctx_tem_perfil('gerente')
    OR (comum.ctx_tem_perfil('medico') AND laudos.ctx_usuario() IN (revisor_id, autor_id))));
CREATE POLICY revisao_designacao ON laudos.revisao FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente') AND criado_por = laudos.ctx_usuario());
CREATE POLICY revisao_atualizacao ON laudos.revisao FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('medico') AND laudos.ctx_usuario() IN (revisor_id, autor_id))
  WITH CHECK (unidade_id = comum.ctx_unidade());

ALTER TABLE laudos.revisao_parecer ENABLE ROW LEVEL SECURITY;
ALTER TABLE laudos.revisao_parecer FORCE ROW LEVEL SECURITY;
CREATE POLICY revisao_parecer_leitura ON laudos.revisao_parecer FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('medico') AND EXISTS (
    SELECT 1 FROM laudos.revisao r WHERE r.id = revisao_id AND laudos.ctx_usuario() IN (r.revisor_id, r.autor_id)));
CREATE POLICY revisao_parecer_escrita ON laudos.revisao_parecer FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('medico') AND criado_por = laudos.ctx_usuario() AND EXISTS (
    SELECT 1 FROM laudos.revisao r WHERE r.id = revisao_id AND r.revisor_id = laudos.ctx_usuario() AND r.situacao = 'concluida'));

GRANT SELECT, INSERT, UPDATE ON laudos.revisao TO vidacor_app;
GRANT SELECT, INSERT ON laudos.revisao_parecer TO vidacor_app;

-- Candidatos ao sorteio: laudos assinados no período, na versão vigente, sem revisão aberta ou
-- concluída nessa versão. Só para a coordenação; devolve só identificadores (sem conteúdo).
CREATE FUNCTION laudos.candidatos_a_revisao(p_desde timestamptz, p_limite int)
RETURNS TABLE (laudo_id uuid, autor_id uuid)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, laudos, comum AS $$
BEGIN
  IF NOT comum.ctx_tem_perfil('gerente') THEN
    RAISE EXCEPTION 'sorteio de revisão só para a coordenação' USING ERRCODE = '42501';
  END IF;
  RETURN QUERY
  SELECT l.id, v.assinada_por
    FROM laudos.laudo l
    JOIN laudos.versao v ON v.laudo_id = l.id AND v.numero = l.versao_vigente AND v.situacao = 'assinada'
   WHERE l.unidade_id = comum.ctx_unidade() AND v.assinada_em >= p_desde
     AND NOT EXISTS (SELECT 1 FROM laudos.revisao r WHERE r.laudo_id = l.id
                      AND (r.situacao = 'designada' OR (r.situacao = 'concluida' AND r.laudo_versao = l.versao_vigente)))
   ORDER BY random()
   LIMIT LEAST(GREATEST(p_limite, 0), 200);
END $$;
REVOKE ALL ON FUNCTION laudos.candidatos_a_revisao(timestamptz, int) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION laudos.candidatos_a_revisao(timestamptz, int) TO vidacor_app;
