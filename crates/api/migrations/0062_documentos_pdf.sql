-- 0062 · Documentos em PDF: laudo impresso e dossiê do lastro (docs/04 Explorador de lastro,
-- docs/21, Laudo-Impresso.dc.html, DEC-2026-11-08-86 a 89). Revisão humana obrigatória antes de
-- rodar fora de dev (G6). Só cria tabelas novas: nada existente é alterado nem apagado.

CREATE SCHEMA documentos;
GRANT USAGE ON SCHEMA documentos TO vidacor_app;

-- ---------------------------------------------------------------------------------------------
-- Cabeçalho impresso nos documentos de cada unidade (endereço, contato, CNPJ, responsável técnico).
-- Configuração versionada: nunca alterada; a versão nova entra vigente e a anterior deixa de ser.
-- Enquanto a clínica não cadastrar, o núcleo imprime só "VidaCor Imagem" e o nome da unidade.
CREATE TABLE documentos.cabecalho (
  id uuid PRIMARY KEY,
  numero int NOT NULL CHECK (numero >= 1),
  nome text NOT NULL CHECK (length(trim(nome)) BETWEEN 2 AND 120),
  endereco text CHECK (length(trim(endereco)) BETWEEN 5 AND 200),
  telefone text CHECK (telefone ~ '^[0-9() +-]{8,20}$'),
  site text CHECK (site ~ '^[a-z0-9.-]{4,80}(/[a-z0-9/_-]{0,60})?$'),
  cnpj text CHECK (cnpj ~ '^[0-9]{2}\.[0-9]{3}\.[0-9]{3}/[0-9]{4}-[0-9]{2}$'),
  responsavel_tecnico text CHECK (length(trim(responsavel_tecnico)) BETWEEN 5 AND 160),
  vigente boolean NOT NULL DEFAULT true,
  motivo text CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  UNIQUE (unidade_id, numero),
  CHECK (numero = 1 OR motivo IS NOT NULL)
);
CREATE UNIQUE INDEX cabecalho_um_vigente ON documentos.cabecalho (unidade_id) WHERE vigente;

-- Registro profissional (conselho, UF, número e RQE) impresso com o nome de quem assina. Uma linha
-- vigente por usuário e conselho; correção entra como linha nova, a antiga deixa de ser vigente.
CREATE TABLE documentos.registro_profissional (
  id uuid PRIMARY KEY,
  usuario_id uuid NOT NULL REFERENCES identidade.usuario(id),
  conselho text NOT NULL CHECK (conselho ~ '^[A-Z]{2,6}$'),
  uf text NOT NULL CHECK (uf ~ '^[A-Z]{2}$'),
  numero text NOT NULL CHECK (numero ~ '^[0-9]{1,10}$'),
  rqe text CHECK (rqe ~ '^[0-9]{1,10}$'),
  vigente boolean NOT NULL DEFAULT true,
  motivo text CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
CREATE UNIQUE INDEX registro_um_vigente ON documentos.registro_profissional (usuario_id, conselho) WHERE vigente;

-- Configuração versionada: a única mudança aceita é deixar de ser vigente; nada é apagado.
CREATE FUNCTION documentos.proteger_versionado() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION '% é versionado: nada é apagado', TG_TABLE_NAME;
  END IF;
  IF NOT (OLD.vigente AND NOT NEW.vigente)
     OR (to_jsonb(NEW) - 'vigente' - 'versao') IS DISTINCT FROM (to_jsonb(OLD) - 'vigente' - 'versao') THEN
    RAISE EXCEPTION '% é versionado: só sai de vigente; a correção é uma versão nova', TG_TABLE_NAME;
  END IF;
  RETURN NEW;
END$$;
CREATE TRIGGER cabecalho_versionado BEFORE UPDATE OR DELETE ON documentos.cabecalho
  FOR EACH ROW EXECUTE FUNCTION documentos.proteger_versionado();
CREATE TRIGGER registro_versionado BEFORE UPDATE OR DELETE ON documentos.registro_profissional
  FOR EACH ROW EXECUTE FUNCTION documentos.proteger_versionado();

-- Leitura pela unidade do contexto (o cabeçalho e o CRM são públicos por natureza, mas a leitura
-- continua limitada à própria unidade). Escrita só pelo núcleo; a tela de Configurações que
-- cadastra estes dados é entrega separada.
ALTER TABLE documentos.cabecalho ENABLE ROW LEVEL SECURITY;
ALTER TABLE documentos.cabecalho FORCE ROW LEVEL SECURITY;
CREATE POLICY cabecalho_leitura ON documentos.cabecalho FOR SELECT
  USING (unidade_id = comum.ctx_unidade());
CREATE POLICY cabecalho_escrita ON documentos.cabecalho FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema'));
CREATE POLICY cabecalho_atualizacao ON documentos.cabecalho FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade());

ALTER TABLE documentos.registro_profissional ENABLE ROW LEVEL SECURITY;
ALTER TABLE documentos.registro_profissional FORCE ROW LEVEL SECURITY;
CREATE POLICY registro_leitura ON documentos.registro_profissional FOR SELECT
  USING (unidade_id = comum.ctx_unidade());
CREATE POLICY registro_escrita ON documentos.registro_profissional FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema'));
CREATE POLICY registro_atualizacao ON documentos.registro_profissional FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade());
GRANT SELECT, INSERT, UPDATE ON documentos.cabecalho, documentos.registro_profissional TO vidacor_app;

-- ---------------------------------------------------------------------------------------------
-- Emissão de documento: cada PDF entregue fica registrado com o SHA-256 dos bytes, o modelo usado,
-- quem pediu e por qual canal. Somente inserção (docs/21: nada é apagado).
CREATE TABLE documentos.emissao (
  id uuid PRIMARY KEY,
  codigo text NOT NULL UNIQUE,
  tipo text NOT NULL CHECK (tipo IN ('laudo_impresso','dossie_lastro')),
  -- Laudo (laudo_impresso) ou atendimento (dossie_lastro).
  registro_id uuid NOT NULL,
  registro_codigo text NOT NULL,
  versao_registro int CHECK (versao_registro >= 1),
  modelo text NOT NULL CHECK (modelo ~ '^[a-z-]+-[0-9]+$'),
  sha256 text NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
  bytes int NOT NULL CHECK (bytes > 0),
  canal text NOT NULL CHECK (canal IN ('equipe','portal')),
  quem uuid NOT NULL,
  quem_tipo text NOT NULL CHECK (quem_tipo IN ('usuario','paciente')),
  criado_em timestamptz NOT NULL DEFAULT now(), origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK ((tipo = 'laudo_impresso') = (versao_registro IS NOT NULL)),
  CHECK (canal = 'equipe' OR (tipo = 'laudo_impresso' AND quem_tipo = 'paciente'))
);
CREATE INDEX emissao_registro_idx ON documentos.emissao (registro_id, criado_em);

CREATE FUNCTION documentos.proibir_alteracao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'documentos.emissao é somente inserção (docs/21)';
END$$;
CREATE TRIGGER emissao_imutavel BEFORE UPDATE OR DELETE ON documentos.emissao
  FOR EACH ROW EXECUTE FUNCTION documentos.proibir_alteracao();

-- Grava só o núcleo (contexto de sistema da unidade); lê a gerência e a auditoria da unidade.
ALTER TABLE documentos.emissao ENABLE ROW LEVEL SECURITY;
ALTER TABLE documentos.emissao FORCE ROW LEVEL SECURITY;
CREATE POLICY emissao_escrita ON documentos.emissao FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema'));
CREATE POLICY emissao_leitura ON documentos.emissao FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema','gerente','auditor'));
GRANT SELECT, INSERT ON documentos.emissao TO vidacor_app;
