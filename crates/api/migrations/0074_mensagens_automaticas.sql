-- 0074 · Comunicação com pacientes (Comunicacao.dc.html), aba "Mensagens automáticas": a gerência liga ou
-- desliga cada aviso automático da unidade, com motivo. Cada mudança é uma versão nova que substitui a
-- anterior (nada se altera nem se apaga; docs/21). Sem versão, o aviso está ligado (como hoje).
-- Aviso desligado não sai e fica registrado como "modelo_desligado" (a tela nunca mente, docs/16).

CREATE TABLE comunicacao.envio_automatico (
  id uuid PRIMARY KEY,
  modelo text NOT NULL CHECK (modelo IN ('confirmacao','laudo_pronto','reserva_expirada','oferta_vaga')),
  ativo boolean NOT NULL,
  motivo text NOT NULL CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  -- Versão que esta substitui (a mudança informa a versão vista: duas mudanças simultâneas, só uma vale).
  substitui uuid UNIQUE REFERENCES comunicacao.envio_automatico(id),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL
);
-- G2: a primeira versão de cada aviso na unidade é única.
CREATE UNIQUE INDEX envio_automatico_primeira ON comunicacao.envio_automatico (unidade_id, modelo) WHERE substitui IS NULL;

CREATE FUNCTION comunicacao.envio_automatico_sem_alteracao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'versão do aviso automático não se altera nem se apaga: registre outra' USING ERRCODE = '42501';
END $$;
CREATE TRIGGER envio_automatico_imutavel BEFORE UPDATE OR DELETE ON comunicacao.envio_automatico
  FOR EACH ROW EXECUTE FUNCTION comunicacao.envio_automatico_sem_alteracao();

ALTER TABLE comunicacao.envio_automatico ENABLE ROW LEVEL SECURITY;
ALTER TABLE comunicacao.envio_automatico FORCE ROW LEVEL SECURITY;
CREATE POLICY envio_automatico_ler ON comunicacao.envio_automatico FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'));
CREATE POLICY envio_automatico_mudar ON comunicacao.envio_automatico FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente') AND criado_por = comum.ctx_usuario());
GRANT SELECT, INSERT ON comunicacao.envio_automatico TO vidacor_app;

ALTER TABLE comunicacao.mensagem DROP CONSTRAINT mensagem_situacao_check;
ALTER TABLE comunicacao.mensagem ADD CONSTRAINT mensagem_situacao_check
  CHECK (situacao IN ('enviada','bloqueada_opt_out','sem_celular','modelo_desligado'));
CREATE INDEX mensagem_unidade_dia_idx ON comunicacao.mensagem (unidade_id, criado_em);
