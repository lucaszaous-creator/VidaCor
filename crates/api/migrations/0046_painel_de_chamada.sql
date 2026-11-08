-- 0046 · Painel de chamada (Telao.dc.html; docs/04 TV de chamada; docs/18: "Dispositivo TV: só o
-- painel de chamada; parcial: senha, nome abreviado, sala").
-- Cada chamada (e cada rechamada) é um registro imutável. O nome abreviado e o destino são gravados
-- pelo núcleo na hora da chamada, para que a TV leia só esta tabela e nunca o cadastro nem o exame.

CREATE TABLE recepcao.chamada (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  atendimento_id uuid NOT NULL REFERENCES recepcao.atendimento(id),
  senha text NOT NULL CHECK (senha ~ '^[A-Z][0-9]{3,4}$'),
  nome_abreviado text NOT NULL CHECK (length(nome_abreviado) BETWEEN 1 AND 40),
  destino_tipo text NOT NULL CHECK (destino_tipo IN ('guiche','sala')),
  guiche int CHECK (guiche BETWEEN 1 AND 20),
  sala_id uuid REFERENCES agenda.sala(id),
  -- Texto mostrado na TV ("Guichê 2", "Sala 3 USG e Doppler"): nome da sala, nunca o exame.
  destino text NOT NULL CHECK (length(destino) BETWEEN 1 AND 80),
  rechamada boolean NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK ((destino_tipo = 'guiche' AND guiche IS NOT NULL AND sala_id IS NULL)
      OR (destino_tipo = 'sala' AND sala_id IS NOT NULL AND guiche IS NULL))
);
CREATE INDEX chamada_painel_idx ON recepcao.chamada (unidade_id, criado_em DESC);

CREATE FUNCTION recepcao.chamada_imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'chamada é registro imutável' USING ERRCODE = '42501'; END $$;
CREATE TRIGGER chamada_sem_alteracao BEFORE UPDATE OR DELETE ON recepcao.chamada
  FOR EACH ROW EXECUTE FUNCTION recepcao.chamada_imutavel();

ALTER TABLE recepcao.chamada ENABLE ROW LEVEL SECURITY;
ALTER TABLE recepcao.chamada FORCE ROW LEVEL SECURITY;
-- Leem: a TV, quem chama e a gerência, na própria unidade. Chamam: recepção (guichê) e técnico
-- (sala em que está lotado).
CREATE POLICY chamada_unidade ON recepcao.chamada
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('tv','recepcao','tecnico','gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','tecnico'));
GRANT SELECT, INSERT ON recepcao.chamada TO vidacor_app;

-- A conta do dispositivo TV tem só esse perfil (separação de funções, docs/18 § 6).
ALTER TABLE identidade.usuario ADD CONSTRAINT usuario_tv_sozinho
  CHECK (NOT ('tv' = ANY(perfis)) OR cardinality(perfis) = 1);
