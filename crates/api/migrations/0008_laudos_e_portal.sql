-- 0008 · Laudo com versões imutáveis, publicação no portal e auditoria de leitura
-- (docs/27 FC-LAU, FC-ACE; CLAUDE.md regras clínicas; Lei 13.787).

CREATE SCHEMA laudos;
CREATE SCHEMA portal;
CREATE SCHEMA auditoria;
GRANT USAGE ON SCHEMA laudos, portal, auditoria TO vidacor_app;

CREATE TABLE laudos.laudo (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  atendimento_id uuid NOT NULL UNIQUE REFERENCES recepcao.atendimento(id),
  estudo_id uuid NOT NULL REFERENCES exames.estudo(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  medico_atribuido_id uuid REFERENCES identidade.usuario(id),
  situacao text NOT NULL CHECK (situacao IN ('rascunho','assinado','retificacao_em_rascunho')),
  versao_vigente int CHECK (versao_vigente >= 1),
  conversar_antes boolean NOT NULL DEFAULT false,
  liberado_apos_conversa_por uuid, liberado_apos_conversa_em timestamptz,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
CREATE INDEX laudo_medico_idx ON laudos.laudo (medico_atribuido_id, situacao);
CREATE INDEX laudo_paciente_idx ON laudos.laudo (paciente_id);

CREATE TABLE laudos.versao (
  id uuid PRIMARY KEY,
  laudo_id uuid NOT NULL REFERENCES laudos.laudo(id),
  numero int NOT NULL CHECK (numero >= 1),
  texto text NOT NULL CHECK (length(texto) <= 100000),
  situacao text NOT NULL CHECK (situacao IN ('rascunho','assinada','substituida')),
  motivo text,
  assinada_por uuid REFERENCES identidade.usuario(id),
  assinada_em timestamptz,
  certificado text, carimbo_tempo timestamptz, hash_documento text,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  UNIQUE (laudo_id, numero),
  CHECK (numero = 1 OR motivo IS NOT NULL),
  CHECK (situacao = 'rascunho' OR (assinada_por IS NOT NULL AND assinada_em IS NOT NULL AND hash_documento IS NOT NULL))
);
-- No máximo uma versão vigente (assinada) por laudo (FC-LAU-07, G2).
CREATE UNIQUE INDEX versao_uma_vigente ON laudos.versao (laudo_id) WHERE situacao = 'assinada';

-- Laudo assinado não muda: texto e assinatura congelados. A única transição permitida de uma versão
-- assinada é para "substituida" quando a retificação é assinada (FC-LAU-04, Lei 13.787).
CREATE FUNCTION laudos.proteger_versao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'versão de laudo não é apagada';
  END IF;
  IF OLD.situacao <> 'rascunho' THEN
    IF NEW.texto IS DISTINCT FROM OLD.texto OR NEW.assinada_por IS DISTINCT FROM OLD.assinada_por
       OR NEW.assinada_em IS DISTINCT FROM OLD.assinada_em OR NEW.hash_documento IS DISTINCT FROM OLD.hash_documento
       OR NOT (OLD.situacao = 'assinada' AND NEW.situacao IN ('assinada','substituida'))
       OR (OLD.situacao = 'substituida' AND NEW.situacao <> 'substituida') THEN
      RAISE EXCEPTION 'versão assinada de laudo é imutável';
    END IF;
  END IF;
  RETURN NEW;
END$$;
CREATE TRIGGER versao_imutavel BEFORE UPDATE OR DELETE ON laudos.versao
  FOR EACH ROW EXECUTE FUNCTION laudos.proteger_versao();

-- Laudo é N4: só médico com vínculo de cuidado ou acesso de emergência, mais o núcleo (sistema).
ALTER TABLE laudos.laudo ENABLE ROW LEVEL SECURITY;
ALTER TABLE laudos.laudo FORCE ROW LEVEL SECURITY;
CREATE POLICY laudo_leitura ON laudos.laudo FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND (
    comum.ctx_tem_perfil('sistema')
    OR (comum.ctx_tem_perfil('medico') AND cuidado.pode_ler(paciente_id))));
CREATE POLICY laudo_escrita ON laudos.laudo FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema'));
CREATE POLICY laudo_atualizacao ON laudos.laudo FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND (comum.ctx_tem_perfil('sistema')
    OR (comum.ctx_tem_perfil('medico') AND medico_atribuido_id = NULLIF(current_setting('app.usuario_id', true), '')::uuid)))
  WITH CHECK (unidade_id = comum.ctx_unidade());

ALTER TABLE laudos.versao ENABLE ROW LEVEL SECURITY;
ALTER TABLE laudos.versao FORCE ROW LEVEL SECURITY;
CREATE POLICY versao_leitura ON laudos.versao FOR SELECT
  USING (EXISTS (SELECT 1 FROM laudos.laudo l WHERE l.id = laudo_id));
CREATE POLICY versao_escrita ON laudos.versao FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('medico','sistema')
    AND EXISTS (SELECT 1 FROM laudos.laudo l WHERE l.id = laudo_id));
CREATE POLICY versao_atualizacao ON laudos.versao FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('medico','sistema')
    AND EXISTS (SELECT 1 FROM laudos.laudo l WHERE l.id = laudo_id))
  WITH CHECK (unidade_id = comum.ctx_unidade());
GRANT SELECT, INSERT, UPDATE ON laudos.laudo, laudos.versao TO vidacor_app;

-- Situação do laudo sem conteúdo, para a recepção e a sala (FC-ACE-06): função que só devolve a
-- palavra da situação, acima do RLS do laudo.
CREATE FUNCTION laudos.situacao_do_atendimento(p_atendimento uuid) RETURNS text LANGUAGE sql STABLE
SECURITY DEFINER SET search_path = pg_catalog, public AS $$
  SELECT l.situacao FROM laudos.laudo l
   WHERE l.atendimento_id = p_atendimento AND l.unidade_id = comum.ctx_unidade()
$$;
GRANT EXECUTE ON FUNCTION laudos.situacao_do_atendimento(uuid) TO vidacor_app;

-- Portal do paciente: só o que foi publicado (laudo assinado e liberado). Uma publicação por versão.
CREATE TABLE portal.conta (
  id uuid PRIMARY KEY, paciente_id uuid NOT NULL UNIQUE REFERENCES identidade.paciente(id),
  login text NOT NULL UNIQUE,
  ativa boolean NOT NULL DEFAULT true,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
CREATE TABLE portal.sessao (
  id uuid PRIMARY KEY, conta_id uuid NOT NULL REFERENCES portal.conta(id),
  segredo_hash bytea NOT NULL UNIQUE, csrf text NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), expira_em timestamptz NOT NULL, revogada_em timestamptz
);
CREATE TABLE portal.publicacao (
  id uuid PRIMARY KEY,
  laudo_id uuid NOT NULL REFERENCES laudos.laudo(id),
  versao_numero int NOT NULL,
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  exame text NOT NULL,
  texto text NOT NULL,
  retificado boolean NOT NULL,
  hash_documento text NOT NULL,
  publicado_em timestamptz NOT NULL DEFAULT now(),
  vigente boolean NOT NULL DEFAULT true,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  UNIQUE (laudo_id, versao_numero)
);
CREATE UNIQUE INDEX publicacao_uma_vigente ON portal.publicacao (laudo_id) WHERE vigente;
-- RLS por paciente no portal (CLAUDE.md G1): o paciente do contexto só vê o que é seu.
ALTER TABLE portal.publicacao ENABLE ROW LEVEL SECURITY;
ALTER TABLE portal.publicacao FORCE ROW LEVEL SECURITY;
CREATE POLICY publicacao_paciente ON portal.publicacao FOR SELECT
  USING (paciente_id = NULLIF(current_setting('app.paciente_id', true), '')::uuid
         OR comum.ctx_tem_perfil('sistema'));
CREATE POLICY publicacao_sistema ON portal.publicacao FOR INSERT
  WITH CHECK (comum.ctx_tem_perfil('sistema'));
CREATE POLICY publicacao_sistema_upd ON portal.publicacao FOR UPDATE
  USING (comum.ctx_tem_perfil('sistema')) WITH CHECK (comum.ctx_tem_perfil('sistema'));
GRANT SELECT ON portal.conta TO vidacor_app;
GRANT SELECT, INSERT, UPDATE ON portal.sessao, portal.publicacao TO vidacor_app;

-- Link compartilhado com médico externo: token aleatório (só o hash no banco), validade curta.
CREATE TABLE portal.link (
  id uuid PRIMARY KEY, token_hash bytea NOT NULL UNIQUE,
  publicacao_id uuid NOT NULL REFERENCES portal.publicacao(id),
  expira_em timestamptz NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
GRANT SELECT, INSERT ON portal.link TO vidacor_app;
-- Leitura pelo link: função que só entrega o laudo se o token existir e estiver válido.
CREATE FUNCTION portal.ler_link(p_hash bytea) RETURNS TABLE (situacao text, exame text, texto text, retificado boolean, publicado_em timestamptz)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public AS $$
  SELECT CASE WHEN k.expira_em > now() THEN 'valido' ELSE 'expirado' END,
         CASE WHEN k.expira_em > now() THEN p.exame END,
         CASE WHEN k.expira_em > now() THEN p.texto END,
         CASE WHEN k.expira_em > now() THEN p.retificado END,
         CASE WHEN k.expira_em > now() THEN p.publicado_em END
    FROM portal.link k JOIN portal.publicacao p ON p.id = k.publicacao_id
   WHERE k.token_hash = p_hash
$$;
GRANT EXECUTE ON FUNCTION portal.ler_link(bytea) TO vidacor_app;

-- Avisos ao paciente: um por publicação (FC-LAU-03).
CREATE TABLE portal.aviso (
  id uuid PRIMARY KEY, publicacao_id uuid NOT NULL UNIQUE REFERENCES portal.publicacao(id),
  canal text NOT NULL, criado_em timestamptz NOT NULL DEFAULT now()
);
GRANT SELECT, INSERT ON portal.aviso TO vidacor_app;

-- Auditoria de leitura de dado clínico e de tentativas negadas (docs/18 § 6 item 9).
CREATE TABLE auditoria.acesso (
  id uuid PRIMARY KEY,
  quem uuid, quem_tipo text NOT NULL CHECK (quem_tipo IN ('usuario','paciente')),
  recurso text NOT NULL, recurso_id uuid,
  resultado text NOT NULL CHECK (resultado IN ('permitido','negado','emergencia')),
  justificativa text,
  criado_em timestamptz NOT NULL DEFAULT now(), correlacao text NOT NULL
);
GRANT INSERT ON auditoria.acesso TO vidacor_app;
GRANT SELECT ON auditoria.acesso TO vidacor_app;
