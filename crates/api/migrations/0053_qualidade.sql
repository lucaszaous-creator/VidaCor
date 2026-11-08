-- 0053 · Qualidade (Qualidade.dc.html): não conformidades e eventos de segurança do paciente.
-- Nada é apagado: andamento é registro novo; o evento muda só de situação. Eventos ficam visíveis só ao
-- Núcleo de Segurança do Paciente (gerência). Dano grave ou óbito: 72 h para notificar a Anvisa
-- (RDC 36/2013) (DEC-2026-11-08-105).
CREATE SCHEMA qualidade;
GRANT USAGE ON SCHEMA qualidade TO vidacor_app;

CREATE TABLE qualidade.nao_conformidade (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  descricao text NOT NULL CHECK (length(trim(descricao)) BETWEEN 10 AND 3000),
  classificacao text NOT NULL CHECK (classificacao IN ('processo','equipamento','documentacao','fornecedor')),
  gravidade text NOT NULL CHECK (gravidade IN ('leve','moderada','grave')),
  responsavel_id uuid REFERENCES identidade.usuario(id),
  prazo_em date,
  situacao text NOT NULL DEFAULT 'aberta' CHECK (situacao IN ('aberta','em_acao','verificacao','encerrada')),
  eficaz boolean,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK ((situacao = 'encerrada') = (eficaz IS NOT NULL))
);
CREATE TABLE qualidade.nc_andamento (
  id uuid PRIMARY KEY,
  nc_id uuid NOT NULL REFERENCES qualidade.nao_conformidade(id),
  texto text NOT NULL CHECK (length(trim(texto)) BETWEEN 5 AND 3000),
  situacao_nova text NOT NULL CHECK (situacao_nova IN ('aberta','em_acao','verificacao','encerrada')),
  versao_da_nc int NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), correlacao text NOT NULL,
  UNIQUE (nc_id, versao_da_nc)
);

CREATE TABLE qualidade.evento (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  tipo text NOT NULL CHECK (tipo IN ('reacao_contraste','queda','erro_identificacao','exame_errado','falha_equipamento','outro')),
  quase_erro boolean NOT NULL,
  gravidade text NOT NULL CHECK (gravidade IN ('sem_dano','leve','moderado','grave','obito')),
  descricao text NOT NULL CHECK (length(trim(descricao)) BETWEEN 10 AND 5000),
  paciente_id uuid REFERENCES identidade.paciente(id),
  anonimo boolean NOT NULL,
  notificar_anvisa_ate timestamptz,
  situacao text NOT NULL DEFAULT 'recebido' CHECK (situacao IN ('recebido','em_analise','concluido')),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK (NOT quase_erro OR gravidade = 'sem_dano'),
  CHECK ((gravidade IN ('grave','obito')) = (notificar_anvisa_ate IS NOT NULL))
);

CREATE FUNCTION qualidade.nada_se_apaga() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'registro de qualidade não se apaga nem se reescreve' USING ERRCODE = '42501'; END $$;
CREATE TRIGGER nc_andamento_imutavel BEFORE UPDATE OR DELETE ON qualidade.nc_andamento FOR EACH ROW EXECUTE FUNCTION qualidade.nada_se_apaga();
CREATE TRIGGER nc_nao_apaga BEFORE DELETE ON qualidade.nao_conformidade FOR EACH ROW EXECUTE FUNCTION qualidade.nada_se_apaga();
CREATE TRIGGER evento_nao_apaga BEFORE DELETE ON qualidade.evento FOR EACH ROW EXECUTE FUNCTION qualidade.nada_se_apaga();
-- O relato do evento não muda depois de notificado; só a situação e a versão.
CREATE FUNCTION qualidade.evento_so_situacao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF (NEW.id, NEW.codigo, NEW.tipo, NEW.quase_erro, NEW.gravidade, NEW.descricao, NEW.paciente_id, NEW.anonimo, NEW.criado_por, NEW.notificar_anvisa_ate)
     IS DISTINCT FROM (OLD.id, OLD.codigo, OLD.tipo, OLD.quase_erro, OLD.gravidade, OLD.descricao, OLD.paciente_id, OLD.anonimo, OLD.criado_por, OLD.notificar_anvisa_ate) THEN
    RAISE EXCEPTION 'evento notificado só muda de situação' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER evento_imutavel BEFORE UPDATE ON qualidade.evento FOR EACH ROW EXECUTE FUNCTION qualidade.evento_so_situacao();

ALTER TABLE qualidade.nao_conformidade ENABLE ROW LEVEL SECURITY;
ALTER TABLE qualidade.nao_conformidade FORCE ROW LEVEL SECURITY;
ALTER TABLE qualidade.nc_andamento ENABLE ROW LEVEL SECURITY;
ALTER TABLE qualidade.nc_andamento FORCE ROW LEVEL SECURITY;
ALTER TABLE qualidade.evento ENABLE ROW LEVEL SECURITY;
ALTER TABLE qualidade.evento FORCE ROW LEVEL SECURITY;
-- Não conformidade: qualquer pessoa da equipe abre (não a TV); a gerência lê e trata.
CREATE POLICY nc_ler ON qualidade.nao_conformidade FOR SELECT USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
CREATE POLICY nc_abrir ON qualidade.nao_conformidade FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND NOT comum.ctx_tem_perfil('tv') AND criado_por = comum.ctx_usuario());
CREATE POLICY nc_tratar ON qualidade.nao_conformidade FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente')) WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
CREATE POLICY nc_andamento_unidade ON qualidade.nc_andamento
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente')) WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
-- Evento: qualquer pessoa da equipe notifica; só o Núcleo de Segurança do Paciente (gerência) lê.
CREATE POLICY evento_ler ON qualidade.evento FOR SELECT USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
CREATE POLICY evento_notificar ON qualidade.evento FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND NOT comum.ctx_tem_perfil('tv') AND criado_por = comum.ctx_usuario());
CREATE POLICY evento_tratar ON qualidade.evento FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente')) WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
GRANT SELECT, INSERT ON qualidade.nao_conformidade, qualidade.nc_andamento, qualidade.evento TO vidacor_app;
GRANT UPDATE (responsavel_id, prazo_em, situacao, eficaz, versao) ON qualidade.nao_conformidade TO vidacor_app;
GRANT UPDATE (situacao, versao) ON qualidade.evento TO vidacor_app;
