-- 0016 · Modo contingência (docs/23, docs/27 FC-CTG-01 a 11). Toda numeração termina usada, anulada
-- ou devolvida; faixas nunca se sobrepõem; papel lançado leva a marca de contingência.

CREATE EXTENSION IF NOT EXISTS btree_gist;
CREATE SCHEMA contingencia;
GRANT USAGE ON SCHEMA contingencia TO vidacor_app;

CREATE TABLE contingencia.contingencia (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  nivel text NOT NULL CHECK (nivel IN ('externo','setor','total')),
  motivo text NOT NULL CHECK (length(trim(motivo)) >= 5),
  declarada_por uuid NOT NULL REFERENCES identidade.usuario(id),
  responsavel_id uuid NOT NULL REFERENCES identidade.usuario(id),
  declarada_em timestamptz NOT NULL,
  -- Chave gerada na estação: a declaração feita sem servidor sobe uma vez só (FC-CTG-02).
  chave uuid NOT NULL UNIQUE,
  situacao text NOT NULL DEFAULT 'aberta' CHECK (situacao IN ('aberta','encerrada')),
  encerrada_em timestamptz, encerrada_por uuid REFERENCES identidade.usuario(id),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
CREATE UNIQUE INDEX contingencia_uma_aberta ON contingencia.contingencia (unidade_id) WHERE situacao = 'aberta';

CREATE TABLE contingencia.kit (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  dia date NOT NULL,
  modelo_versao text NOT NULL,
  conteudo text NOT NULL,
  hash text NOT NULL CHECK (hash ~ '^[0-9a-f]{64}$'),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  UNIQUE (unidade_id, dia)
);

CREATE TABLE contingencia.faixa (
  id uuid PRIMARY KEY,
  kit_id uuid NOT NULL REFERENCES contingencia.kit(id),
  formulario text NOT NULL CHECK (formulario IN ('F-ATD','F-REC')),
  dia date NOT NULL,
  inicio int NOT NULL CHECK (inicio >= 1),
  fim int NOT NULL CHECK (fim >= inicio AND fim <= 9999),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  -- Faixas do mesmo formulário nunca se sobrepõem (FC-CTG-11, G2 no banco).
  EXCLUDE USING gist (unidade_id WITH =, formulario WITH =, int4range(inicio, fim, '[]') WITH &&)
);

-- Situação de cada número usado, anulado ou devolvido. Número sem linha aqui está "sem situação".
CREATE TABLE contingencia.numero (
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  formulario text NOT NULL,
  numero int NOT NULL,
  faixa_id uuid NOT NULL REFERENCES contingencia.faixa(id),
  contingencia_id uuid REFERENCES contingencia.contingencia(id),
  situacao text NOT NULL CHECK (situacao IN ('usado','pendente','anulado','devolvido')),
  registro_id uuid,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL DEFAULT 'contingencia',
  estacao text, versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  -- Mesma ficha lançada duas vezes: um registro só (FC-CTG-04).
  PRIMARY KEY (unidade_id, formulario, numero)
);

CREATE TABLE contingencia.recibo (
  id uuid PRIMARY KEY, numero int NOT NULL,
  contingencia_id uuid NOT NULL REFERENCES contingencia.contingencia(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  tuss text NOT NULL,
  valor_papel_centavos bigint NOT NULL CHECK (valor_papel_centavos >= 0),
  valor_recalculado_centavos bigint NOT NULL CHECK (valor_recalculado_centavos > 0),
  tabela_versao int NOT NULL,
  ocorrido_em timestamptz NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);

-- Atendimento lançado do papel: marca de contingência, ficha, quando ocorreu, quem atendeu, hash do PDF.
ALTER TABLE recepcao.atendimento
  ADD COLUMN contingencia_id uuid REFERENCES contingencia.contingencia(id),
  ADD COLUMN formulario text,
  ADD COLUMN ocorrido_em timestamptz,
  ADD COLUMN atendido_por uuid REFERENCES identidade.usuario(id),
  ADD COLUMN anexo_sha256 text CHECK (anexo_sha256 IS NULL OR anexo_sha256 ~ '^[0-9a-f]{64}$'),
  ADD CONSTRAINT atendimento_contingencia_completo CHECK (
    contingencia_id IS NULL OR (formulario IS NOT NULL AND ocorrido_em IS NOT NULL AND anexo_sha256 IS NOT NULL));
ALTER TABLE recepcao.divergencia ADD COLUMN contingencia_id uuid REFERENCES contingencia.contingencia(id);

-- Estudo de contingência (Patient ID "C-NNNN") vai para a fila própria de conciliação (FC-CTG-08).
ALTER TABLE exames.estudo ADD COLUMN pendencia text CHECK (pendencia IN ('conciliacao','conciliacao_contingencia'));

-- RLS: a contingência e seus registros são operacionais da unidade (sem dado clínico).
ALTER TABLE contingencia.contingencia ENABLE ROW LEVEL SECURITY;
ALTER TABLE contingencia.contingencia FORCE ROW LEVEL SECURITY;
CREATE POLICY contingencia_unidade ON contingencia.contingencia
  USING (unidade_id = comum.ctx_unidade()) WITH CHECK (unidade_id = comum.ctx_unidade());
ALTER TABLE contingencia.kit ENABLE ROW LEVEL SECURITY;
ALTER TABLE contingencia.kit FORCE ROW LEVEL SECURITY;
CREATE POLICY kit_unidade ON contingencia.kit
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade());
ALTER TABLE contingencia.faixa ENABLE ROW LEVEL SECURITY;
ALTER TABLE contingencia.faixa FORCE ROW LEVEL SECURITY;
CREATE POLICY faixa_unidade ON contingencia.faixa USING (unidade_id = comum.ctx_unidade()) WITH CHECK (unidade_id = comum.ctx_unidade());
ALTER TABLE contingencia.numero ENABLE ROW LEVEL SECURITY;
ALTER TABLE contingencia.numero FORCE ROW LEVEL SECURITY;
CREATE POLICY numero_unidade ON contingencia.numero USING (unidade_id = comum.ctx_unidade()) WITH CHECK (unidade_id = comum.ctx_unidade());
ALTER TABLE contingencia.recibo ENABLE ROW LEVEL SECURITY;
ALTER TABLE contingencia.recibo FORCE ROW LEVEL SECURITY;
CREATE POLICY recibo_unidade ON contingencia.recibo
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','financeiro','caixa'))
  WITH CHECK (unidade_id = comum.ctx_unidade());
GRANT SELECT, INSERT, UPDATE ON contingencia.contingencia, contingencia.kit, contingencia.faixa, contingencia.numero, contingencia.recibo TO vidacor_app;
