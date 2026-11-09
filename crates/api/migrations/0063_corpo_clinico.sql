-- 0063 · Corpo clínico e cabeçalho da unidade (Corpo-Clinico.dc.html, Configuracoes.dc.html;
-- docs/04 Corpo clínico: "médico novo e mudanças só valem após segunda conferência"; docs/18 § 6;
-- DEC-2026-11-08-91 e 92). Revisão humana obrigatória antes de rodar fora de dev (G6).
-- Só acrescenta: colunas novas anuláveis, tabelas novas, políticas e gatilhos novos. Nada existente é
-- alterado, afrouxado nem apagado.

CREATE SCHEMA corpo;
GRANT USAGE ON SCHEMA corpo TO vidacor_app;

-- ---------------------------------------------------------------------------------------------
-- Pedido de mudança no cadastro de um médico (registro profissional ou o que realiza e lauda).
-- Só vale depois da conferência de OUTRA pessoa da gerência: quem pede não decide, e o próprio
-- médico não decide. Um pedido aguardando por médico e tipo (G2).
CREATE TABLE corpo.pedido (
  id uuid PRIMARY KEY,
  codigo text NOT NULL UNIQUE,
  medico_id uuid NOT NULL REFERENCES identidade.usuario(id),
  tipo text NOT NULL CHECK (tipo IN ('registro','habilitacao')),
  dados jsonb NOT NULL CHECK (jsonb_typeof(dados) = 'object'),
  motivo text NOT NULL CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  situacao text NOT NULL DEFAULT 'aguardando' CHECK (situacao IN ('aguardando','aplicado','recusado')),
  pedido_por uuid NOT NULL REFERENCES identidade.usuario(id),
  decidido_por uuid REFERENCES identidade.usuario(id),
  decidido_em timestamptz,
  motivo_decisao text CHECK (length(trim(motivo_decisao)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK (pedido_por <> medico_id),
  CHECK ((situacao = 'aguardando') = (decidido_por IS NULL)),
  CHECK (decidido_por IS NULL OR (decidido_por <> pedido_por AND decidido_por <> medico_id)),
  CHECK (situacao = 'aguardando' OR (decidido_em IS NOT NULL AND motivo_decisao IS NOT NULL))
);
CREATE UNIQUE INDEX pedido_corpo_um_aguardando ON corpo.pedido (medico_id, tipo) WHERE situacao = 'aguardando';
CREATE INDEX pedido_corpo_medico_idx ON corpo.pedido (medico_id, criado_em);

-- A única mudança aceita no pedido é a decisão (aguardando para aplicado ou recusado), uma vez.
CREATE FUNCTION corpo.pedido_so_decide() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'pedido do corpo clínico não se apaga' USING ERRCODE = '42501';
  END IF;
  IF OLD.situacao <> 'aguardando' OR NEW.situacao = 'aguardando'
     OR (to_jsonb(NEW) - 'situacao' - 'decidido_por' - 'decidido_em' - 'motivo_decisao' - 'versao')
        IS DISTINCT FROM (to_jsonb(OLD) - 'situacao' - 'decidido_por' - 'decidido_em' - 'motivo_decisao' - 'versao') THEN
    RAISE EXCEPTION 'pedido do corpo clínico só recebe a decisão, uma vez' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER pedido_corpo_so_decide BEFORE UPDATE OR DELETE ON corpo.pedido
  FOR EACH ROW EXECUTE FUNCTION corpo.pedido_so_decide();

ALTER TABLE corpo.pedido ENABLE ROW LEVEL SECURITY;
ALTER TABLE corpo.pedido FORCE ROW LEVEL SECURITY;
CREATE POLICY pedido_corpo_gerencia ON corpo.pedido
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
GRANT SELECT, INSERT ON corpo.pedido TO vidacor_app;
GRANT UPDATE (situacao, decidido_por, decidido_em, motivo_decisao, versao) ON corpo.pedido TO vidacor_app;

-- ---------------------------------------------------------------------------------------------
-- O que o médico realiza na sala e o que lauda, por modalidade (US, CT, DX...). Configuração
-- versionada: a versão nova entra vigente e a anterior deixa de ser (mesma proteção da 0062).
CREATE TABLE corpo.habilitacao (
  id uuid PRIMARY KEY,
  usuario_id uuid NOT NULL REFERENCES identidade.usuario(id),
  realiza text[] NOT NULL CHECK (cardinality(realiza) <= 12
    AND array_to_string(realiza, ',') ~ '^([A-Z]{2,4}(,[A-Z]{2,4})*)?$'),
  lauda text[] NOT NULL CHECK (cardinality(lauda) <= 12
    AND array_to_string(lauda, ',') ~ '^([A-Z]{2,4}(,[A-Z]{2,4})*)?$'),
  pedido_id uuid NOT NULL UNIQUE REFERENCES corpo.pedido(id),
  vigente boolean NOT NULL DEFAULT true,
  motivo text NOT NULL CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL
);
CREATE UNIQUE INDEX habilitacao_uma_vigente ON corpo.habilitacao (usuario_id) WHERE vigente;
CREATE TRIGGER habilitacao_versionada BEFORE UPDATE OR DELETE ON corpo.habilitacao
  FOR EACH ROW EXECUTE FUNCTION documentos.proteger_versionado();
ALTER TABLE corpo.habilitacao ENABLE ROW LEVEL SECURITY;
ALTER TABLE corpo.habilitacao FORCE ROW LEVEL SECURITY;
CREATE POLICY habilitacao_gerencia ON corpo.habilitacao
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
GRANT SELECT, INSERT, UPDATE ON corpo.habilitacao TO vidacor_app;

-- ---------------------------------------------------------------------------------------------
-- Registro profissional (0062): especialidade junto do RQE e o pedido conferido que originou a
-- versão. As linhas antigas (sem pedido) continuam válidas.
ALTER TABLE documentos.registro_profissional
  ADD COLUMN especialidade text CHECK (length(trim(especialidade)) BETWEEN 3 AND 80),
  ADD COLUMN pedido_id uuid UNIQUE REFERENCES corpo.pedido(id);
-- Um CRM vigente pertence a um só médico da unidade (G2).
CREATE UNIQUE INDEX registro_numero_unico ON documentos.registro_profissional (unidade_id, conselho, uf, numero) WHERE vigente;

-- A gerência grava a versão nova (e tira a anterior de vigente) só na própria unidade. Políticas
-- novas, somadas às de 0062 (escrita do núcleo em contexto de sistema continua igual).
CREATE POLICY registro_gerencia_escrita ON documentos.registro_profissional FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
CREATE POLICY registro_gerencia_atualizacao ON documentos.registro_profissional FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade());

-- Segunda barreira da segunda conferência: pelo papel da aplicação (fora do contexto de sistema),
-- versão nova de registro ou habilitação só entra com um pedido do mesmo médico já aplicado por
-- quem está gravando, e esse alguém não é quem pediu.
CREATE FUNCTION corpo.versao_conferida() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF current_user = 'vidacor_app' AND NOT comum.ctx_tem_perfil('sistema') THEN
    IF NOT EXISTS (
      SELECT 1 FROM corpo.pedido p
       WHERE p.id = NEW.pedido_id AND p.medico_id = NEW.usuario_id AND p.situacao = 'aplicado'
         AND p.decidido_por = comum.ctx_usuario() AND p.pedido_por <> comum.ctx_usuario()
    ) THEN
      RAISE EXCEPTION 'versão do corpo clínico sem segunda conferência' USING ERRCODE = '42501';
    END IF;
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER registro_conferido BEFORE INSERT ON documentos.registro_profissional
  FOR EACH ROW EXECUTE FUNCTION corpo.versao_conferida();
CREATE TRIGGER habilitacao_conferida BEFORE INSERT ON corpo.habilitacao
  FOR EACH ROW EXECUTE FUNCTION corpo.versao_conferida();

-- ---------------------------------------------------------------------------------------------
-- Cabeçalho da unidade (0062): razão social e o responsável técnico em campos próprios (o texto
-- impresso, `responsavel_tecnico`, continua sendo o que o PDF lê). A gerência grava a versão nova.
ALTER TABLE documentos.cabecalho
  ADD COLUMN razao_social text CHECK (length(trim(razao_social)) BETWEEN 2 AND 160),
  ADD COLUMN responsavel_nome text CHECK (length(trim(responsavel_nome)) BETWEEN 5 AND 120),
  ADD COLUMN responsavel_crm text CHECK (responsavel_crm ~ '^CRM-[A-Z]{2} [0-9]{1,10}$');
CREATE POLICY cabecalho_gerencia_escrita ON documentos.cabecalho FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
CREATE POLICY cabecalho_gerencia_atualizacao ON documentos.cabecalho FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade());
