-- 0020 · Cadastro de paciente (Cadastro-Paciente.dc.html, docs/04): identificação, contato e alergias.
-- O mínimo para agendar é nome, nascimento, sexo e celular; alergias informadas ou negadas. CPF é
-- opcional e único. Cadastros antigos (semente) ficam com os campos novos vazios: nada é inventado.

ALTER TABLE identidade.paciente
  ADD COLUMN nome_social text CHECK (nome_social IS NULL OR length(trim(nome_social)) BETWEEN 2 AND 200),
  ADD COLUMN cpf text CHECK (cpf IS NULL OR cpf ~ '^[0-9]{11}$'),
  ADD COLUMN nome_mae text CHECK (nome_mae IS NULL OR length(trim(nome_mae)) BETWEEN 3 AND 200),
  ADD COLUMN celular text CHECK (celular IS NULL OR celular ~ '^[0-9]{10,11}$'),
  ADD COLUMN email text CHECK (email IS NULL OR email ~ '^[^@\s]{1,64}@[^@\s]{1,190}$'),
  ADD COLUMN alergias text[],
  ADD COLUMN nega_alergias boolean,
  ADD CONSTRAINT paciente_alergias_coerentes CHECK (NOT (coalesce(nega_alergias, false) AND cardinality(coalesce(alergias, '{}')) > 0));
-- Um CPF, um cadastro (G2: duas recepções cadastrando a mesma pessoa ao mesmo tempo).
CREATE UNIQUE INDEX paciente_cpf_unico ON identidade.paciente (cpf) WHERE cpf IS NOT NULL;
CREATE INDEX paciente_nascimento_idx ON identidade.paciente (unidade_id, nascimento);

-- Cadastros que já existem (semente, migração dos sistemas antigos) ocupam códigos PAC-AAAA-NNNNNN: o
-- contador começa depois do maior número de cada ano, para o cadastro novo não colidir.
INSERT INTO comum.contador (prefixo, ano, valor)
SELECT 'PAC', substring(codigo FROM 5 FOR 4)::int, max(substring(codigo FROM 10)::bigint)
  FROM identidade.paciente WHERE codigo ~ '^PAC-[0-9]{4}-[0-9]{6}$' GROUP BY 2
ON CONFLICT (prefixo, ano) DO UPDATE SET valor = GREATEST(comum.contador.valor, EXCLUDED.valor);
