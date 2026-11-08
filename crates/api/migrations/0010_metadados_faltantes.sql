-- 0010 · Metadados de lastro que faltavam (docs/21 § 5 item 1, achado pelo teste de schema).
-- Procedimentos do agendamento são registro de negócio: levam os metadados.
ALTER TABLE agenda.agendamento_procedimento
  ADD COLUMN criado_em timestamptz NOT NULL DEFAULT now(),
  ADD COLUMN criado_por uuid,
  ADD COLUMN origem text NOT NULL DEFAULT 'nucleo',
  ADD COLUMN estacao text,
  ADD COLUMN unidade_id uuid REFERENCES identidade.unidade(id),
  ADD COLUMN versao int NOT NULL DEFAULT 1,
  ADD COLUMN correlacao text NOT NULL DEFAULT '';
UPDATE agenda.agendamento_procedimento p SET unidade_id = a.unidade_id
  FROM agenda.agendamento a WHERE a.id = p.agendamento_id;
ALTER TABLE agenda.agendamento_procedimento ALTER COLUMN unidade_id SET NOT NULL;

-- A senha do dia é contador técnico, como comum.contador: vai para o schema comum.
ALTER TABLE recepcao.senha_dia SET SCHEMA comum;
CREATE OR REPLACE FUNCTION recepcao.proxima_senha(p_unidade uuid, p_dia date) RETURNS int LANGUAGE sql AS $$
  INSERT INTO comum.senha_dia AS s (unidade_id, dia, valor) VALUES (p_unidade, p_dia, 1)
  ON CONFLICT (unidade_id, dia) DO UPDATE SET valor = s.valor + 1
  RETURNING valor
$$;
