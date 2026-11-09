-- 0072 · Lista de pacientes (Pacientes-Lista.dc.html) e conferência de possíveis duplicados.
-- A recepção decide sobre um par de cadastros parecidos: "pedir junção" (aguarda a conferência de outra
-- pessoa; até lá os cadastros seguem separados) ou "são pessoas diferentes" (o par sai da lista). A
-- decisão é um registro imutável; mudar de ideia é nova decisão que substitui a anterior, com motivo
-- (docs/16 R5, docs/21). A junção em si (mover agendamentos, laudos e imagens para um só cadastro)
-- NÃO é feita aqui: é ato clínico de risco alto e aguarda o responsável clínico (DEC-2026-11-09-127).

CREATE TABLE identidade.par_duplicado (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  -- O par fica sempre na mesma ordem (menor, maior) para que A-B e B-A sejam o mesmo par.
  paciente_menor uuid NOT NULL REFERENCES identidade.paciente(id),
  paciente_maior uuid NOT NULL REFERENCES identidade.paciente(id),
  decisao text NOT NULL CHECK (decisao IN ('juncao_pedida','pessoas_diferentes')),
  motivo text CHECK (motivo IS NULL OR length(trim(motivo)) BETWEEN 5 AND 500),
  -- Decisão que esta substitui (correção com motivo, nunca alteração da anterior).
  substitui uuid UNIQUE REFERENCES identidade.par_duplicado(id),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK (paciente_menor < paciente_maior),
  CHECK (substitui IS NULL OR motivo IS NOT NULL)
);
-- G2: a primeira decisão de um par é única (duas recepções decidindo ao mesmo tempo: só uma vale).
CREATE UNIQUE INDEX par_duplicado_primeira ON identidade.par_duplicado (paciente_menor, paciente_maior)
  WHERE substitui IS NULL;
CREATE INDEX par_duplicado_paciente_maior_idx ON identidade.par_duplicado (paciente_maior);

CREATE FUNCTION identidade.par_duplicado_sem_alteracao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'decisão sobre duplicado não se altera nem se apaga: registre outra com motivo' USING ERRCODE = '42501';
END $$;
CREATE TRIGGER par_duplicado_imutavel BEFORE UPDATE OR DELETE ON identidade.par_duplicado
  FOR EACH ROW EXECUTE FUNCTION identidade.par_duplicado_sem_alteracao();

ALTER TABLE identidade.par_duplicado ENABLE ROW LEVEL SECURITY;
ALTER TABLE identidade.par_duplicado FORCE ROW LEVEL SECURITY;
CREATE POLICY par_duplicado_ler ON identidade.par_duplicado FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente'));
CREATE POLICY par_duplicado_decidir ON identidade.par_duplicado FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente')
              AND criado_por = comum.ctx_usuario());
GRANT SELECT, INSERT ON identidade.par_duplicado TO vidacor_app;

-- Busca da lista por nome sem diferenciar maiúsculas nem acentos: a recepção digita "antonio" e acha
-- "Antônio" (DEC-2026-11-09-125). Função imutável para poder ser indexada.
CREATE FUNCTION comum.sem_acento(t text) RETURNS text LANGUAGE sql IMMUTABLE PARALLEL SAFE AS
$$ SELECT translate(lower(t), 'áàâãäéèêëíìîïóòôõöúùûüçñ', 'aaaaaeeeeiiiiooooouuuucn') $$;
CREATE INDEX paciente_nome_sem_acento_idx ON identidade.paciente (unidade_id, comum.sem_acento(nome));
CREATE INDEX paciente_celular_idx ON identidade.paciente (unidade_id, celular) WHERE celular IS NOT NULL;
