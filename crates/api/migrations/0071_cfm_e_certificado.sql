-- 0071 · Corpo clínico: consulta do CRM no CFM e certificado ICP-Brasil do médico (Corpo-Clinico.dc.html;
-- DEC-2026-11-09-120 e 113). Revisão humana obrigatória antes de rodar fora de dev (G6). Só cria
-- tabelas novas, gatilhos e políticas; nada existente é alterado nem apagado.

-- ---------------------------------------------------------------------------------------------
-- Consulta do CRM no conselho: pelo webservice do CFM (com o hash da resposta como comprovante) ou
-- feita pela gerência no portal do CFM (com a observação de onde e como). Somente inserção.
CREATE TABLE corpo.consulta_crm (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  medico_id uuid NOT NULL REFERENCES identidade.usuario(id),
  uf text NOT NULL CHECK (uf ~ '^[A-Z]{2}$'),
  numero text NOT NULL CHECK (numero ~ '^[0-9]{1,10}$'),
  fonte text NOT NULL CHECK (fonte IN ('webservice_cfm','portal_cfm')),
  situacao text NOT NULL CHECK (situacao IN
    ('regular','suspenso','cassado','cancelado','falecido','transferido','interdicao_cautelar','nao_encontrado')),
  nome_no_conselho text CHECK (length(trim(nome_no_conselho)) BETWEEN 3 AND 160),
  especialidades text[] NOT NULL DEFAULT '{}',
  resposta_sha256 text CHECK (resposta_sha256 ~ '^[0-9a-f]{64}$'),
  observacao text CHECK (length(trim(observacao)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  -- Webservice guarda o comprovante; consulta no portal diz onde e como foi feita.
  CHECK (fonte <> 'webservice_cfm' OR resposta_sha256 IS NOT NULL),
  CHECK (fonte <> 'portal_cfm' OR observacao IS NOT NULL)
);
CREATE INDEX consulta_crm_medico_idx ON corpo.consulta_crm (medico_id, criado_em);

CREATE FUNCTION corpo.somente_insercao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION '% é somente inserção (docs/21)', TG_TABLE_NAME USING ERRCODE = '42501'; END $$;
CREATE TRIGGER consulta_crm_imutavel BEFORE UPDATE OR DELETE ON corpo.consulta_crm
  FOR EACH ROW EXECUTE FUNCTION corpo.somente_insercao();

ALTER TABLE corpo.consulta_crm ENABLE ROW LEVEL SECURITY;
ALTER TABLE corpo.consulta_crm FORCE ROW LEVEL SECURITY;
CREATE POLICY consulta_crm_gerencia ON corpo.consulta_crm
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
GRANT SELECT, INSERT ON corpo.consulta_crm TO vidacor_app;

-- ---------------------------------------------------------------------------------------------
-- Certificado ICP-Brasil do médico: só os dados do certificado PÚBLICO (nunca chave privada).
-- Versionado: o novo entra vigente e o anterior só deixa de ser (gatilho de 0062).
CREATE TABLE corpo.certificado (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  medico_id uuid NOT NULL REFERENCES identidade.usuario(id),
  titular text NOT NULL CHECK (length(trim(titular)) BETWEEN 2 AND 200),
  cpf_mascarado text CHECK (cpf_mascarado ~ '^\*\*\*\.\*\*\*\.\*\*\*-[0-9]{2}$'),
  emissor text NOT NULL CHECK (length(emissor) BETWEEN 2 AND 500),
  serie text NOT NULL CHECK (length(serie) BETWEEN 1 AND 200),
  tipo text CHECK (tipo IN ('A1','A2','A3','A4','outro')),
  valido_de timestamptz NOT NULL,
  valido_ate timestamptz NOT NULL CHECK (valido_ate > valido_de),
  sha256 text NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
  vigente boolean NOT NULL DEFAULT true,
  motivo text CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL
);
CREATE UNIQUE INDEX certificado_um_vigente ON corpo.certificado (medico_id) WHERE vigente;
CREATE TRIGGER certificado_versionado BEFORE UPDATE OR DELETE ON corpo.certificado
  FOR EACH ROW EXECUTE FUNCTION documentos.proteger_versionado();

ALTER TABLE corpo.certificado ENABLE ROW LEVEL SECURITY;
ALTER TABLE corpo.certificado FORCE ROW LEVEL SECURITY;
CREATE POLICY certificado_gerencia ON corpo.certificado
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
GRANT SELECT, INSERT, UPDATE ON corpo.certificado TO vidacor_app;
