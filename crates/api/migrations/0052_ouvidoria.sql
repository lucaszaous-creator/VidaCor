-- 0052 · Ouvidoria (Ouvidoria.dc.html): manifestações de qualquer canal com prazo calculado pelo
-- núcleo. O relato pode ter dado de saúde: só a gerência (ouvidoria) lê; a recepção registra. Resposta
-- não se apaga nem se altera; encaminhar e responder são eventos (DEC-2026-11-08-99).
CREATE SCHEMA ouvidoria;
GRANT USAGE ON SCHEMA ouvidoria TO vidacor_app;

CREATE TABLE ouvidoria.manifestacao (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  tipo text NOT NULL CHECK (tipo IN ('reclamacao','elogio','sugestao','denuncia','lgpd')),
  canal text NOT NULL CHECK (canal IN ('telefone','presencial','email','portal')),
  paciente_id uuid REFERENCES identidade.paciente(id),
  -- Quem não é paciente do cadastro (acompanhante, familiar) ou deixou só o nome.
  nome_informado text CHECK (nome_informado IS NULL OR length(trim(nome_informado)) BETWEEN 2 AND 200),
  anonima boolean NOT NULL DEFAULT false,
  relato text NOT NULL CHECK (length(trim(relato)) BETWEEN 10 AND 5000),
  recebida_em date NOT NULL,
  prazo_em date NOT NULL,
  situacao text NOT NULL DEFAULT 'aberta' CHECK (situacao IN ('aberta','respondida')),
  resposta text CHECK (resposta IS NULL OR length(trim(resposta)) BETWEEN 20 AND 5000),
  respondida_por uuid REFERENCES identidade.usuario(id),
  respondida_em timestamptz,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK (prazo_em >= recebida_em),
  -- Anônima só para denúncia, e sem identificação; as demais pedem paciente ou nome.
  CHECK (NOT anonima OR (tipo = 'denuncia' AND paciente_id IS NULL AND nome_informado IS NULL)),
  CHECK (anonima OR paciente_id IS NOT NULL OR nome_informado IS NOT NULL),
  -- Pedido LGPD é do titular: precisa do paciente do cadastro.
  CHECK (tipo <> 'lgpd' OR paciente_id IS NOT NULL),
  CHECK ((situacao = 'respondida') = (resposta IS NOT NULL AND respondida_em IS NOT NULL AND respondida_por IS NOT NULL))
);
CREATE INDEX manifestacao_aberta_idx ON ouvidoria.manifestacao (unidade_id, situacao, prazo_em);

-- Só a passagem aberta -> respondida (uma vez); nada se apaga.
CREATE FUNCTION ouvidoria.manifestacao_sem_alteracao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN RAISE EXCEPTION 'manifestação não se apaga' USING ERRCODE = '42501'; END IF;
  IF OLD.situacao <> 'aberta' OR NEW.situacao <> 'respondida'
     OR (NEW.id, NEW.codigo, NEW.tipo, NEW.relato, NEW.paciente_id, NEW.prazo_em, NEW.recebida_em, NEW.criado_por)
        IS DISTINCT FROM (OLD.id, OLD.codigo, OLD.tipo, OLD.relato, OLD.paciente_id, OLD.prazo_em, OLD.recebida_em, OLD.criado_por) THEN
    RAISE EXCEPTION 'manifestação só muda de aberta para respondida' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER manifestacao_imutavel BEFORE UPDATE OR DELETE ON ouvidoria.manifestacao
  FOR EACH ROW EXECUTE FUNCTION ouvidoria.manifestacao_sem_alteracao();

ALTER TABLE ouvidoria.manifestacao ENABLE ROW LEVEL SECURITY;
ALTER TABLE ouvidoria.manifestacao FORCE ROW LEVEL SECURITY;
-- A recepção registra (INSERT) mas não lê a lista; a gerência lê e responde.
CREATE POLICY manifestacao_ler ON ouvidoria.manifestacao FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
CREATE POLICY manifestacao_registrar ON ouvidoria.manifestacao FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente') AND criado_por = comum.ctx_usuario());
CREATE POLICY manifestacao_responder ON ouvidoria.manifestacao FOR UPDATE
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
GRANT SELECT, INSERT ON ouvidoria.manifestacao TO vidacor_app;
GRANT UPDATE (situacao, resposta, respondida_por, respondida_em, versao) ON ouvidoria.manifestacao TO vidacor_app;
