-- 0007 · Triagem com contraste (docs/22 E1, docs/27 FC-TRI) e vínculo de cuidado (docs/18 § 6).

-- Vínculo de cuidado: o médico lê dado clínico completo só com vínculo (agendamento, exame,
-- laudo atribuído, atendimento). Expira; laudo assinado por ele mantém o vínculo.
CREATE SCHEMA cuidado;
GRANT USAGE ON SCHEMA cuidado TO vidacor_app;
CREATE TABLE cuidado.vinculo (
  medico_id uuid NOT NULL REFERENCES identidade.usuario(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  motivo text NOT NULL CHECK (motivo IN ('agendamento','laudo_atribuido','laudo_assinado','atendimento')),
  expira_em timestamptz NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  PRIMARY KEY (medico_id, paciente_id, motivo)
);
GRANT SELECT, INSERT, UPDATE ON cuidado.vinculo TO vidacor_app;

-- Acesso de emergência (break-glass): justificativa obrigatória, vale 1 hora, revisão em 48 h.
CREATE TABLE cuidado.acesso_emergencia (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  medico_id uuid NOT NULL REFERENCES identidade.usuario(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  justificativa text NOT NULL CHECK (length(trim(justificativa)) >= 20),
  valido_ate timestamptz NOT NULL,
  revisar_ate timestamptz NOT NULL,
  revisado_em timestamptz, revisado_por uuid,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
CREATE INDEX acesso_emergencia_medico_idx ON cuidado.acesso_emergencia (medico_id, paciente_id, valido_ate);
GRANT SELECT, INSERT ON cuidado.acesso_emergencia TO vidacor_app;

-- O médico do contexto tem vínculo ou acesso de emergência ativo com o paciente?
CREATE FUNCTION cuidado.pode_ler(p_paciente uuid) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
  SELECT EXISTS (
    SELECT 1 FROM cuidado.vinculo v
     WHERE v.paciente_id = p_paciente
       AND v.medico_id = NULLIF(current_setting('app.usuario_id', true), '')::uuid
       AND v.expira_em > now())
  OR EXISTS (
    SELECT 1 FROM cuidado.acesso_emergencia a
     WHERE a.paciente_id = p_paciente
       AND a.medico_id = NULLIF(current_setting('app.usuario_id', true), '')::uuid
       AND a.valido_ate > now())
$$;
GRANT EXECUTE ON FUNCTION cuidado.pode_ler(uuid) TO vidacor_app;

-- Sexo e idade vêm do cadastro; a regra é versionada e gravada em cada triagem.
CREATE SCHEMA triagem;
GRANT USAGE ON SCHEMA triagem TO vidacor_app;
CREATE TABLE triagem.triagem (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  atendimento_id uuid NOT NULL REFERENCES recepcao.atendimento(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  reacao_previa_contraste boolean NOT NULL,
  gestante boolean NOT NULL,
  amamentando boolean NOT NULL,
  usa_metformina boolean NOT NULL,
  creatinina_mg_dl numeric(5,2) CHECK (creatinina_mg_dl > 0 AND creatinina_mg_dl < 30),
  creatinina_data date,
  tfg_calculada int CHECK (tfg_calculada >= 0),
  resultado text NOT NULL CHECK (resultado IN ('liberado','liberado_com_ressalvas','precisa_aprovacao_medica')),
  motivos text[] NOT NULL DEFAULT '{}',
  regra_versao text NOT NULL,
  conduta text,
  conduta_por uuid REFERENCES identidade.usuario(id),
  conduta_em timestamptz,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK (conduta IS NULL OR (conduta_por IS NOT NULL AND conduta_em IS NOT NULL))
);
-- Uma triagem vigente por atendimento (nova triagem vira nova versão, controlada pelo núcleo).
CREATE UNIQUE INDEX triagem_uma_por_atendimento ON triagem.triagem (atendimento_id);
ALTER TABLE triagem.triagem ENABLE ROW LEVEL SECURITY;
ALTER TABLE triagem.triagem FORCE ROW LEVEL SECURITY;
-- Triagem é N3 (clínico resumido): técnico, médico e recepção (preparo) da unidade.
CREATE POLICY triagem_unidade ON triagem.triagem
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('tecnico','medico'));
GRANT SELECT, INSERT, UPDATE ON triagem.triagem TO vidacor_app;
