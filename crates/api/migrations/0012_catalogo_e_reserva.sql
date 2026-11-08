-- 0012 · Catálogo de procedimentos e expiração de reserva pelo núcleo (docs/27 FC-AGD, docs/30 § 3).
-- Códigos e nomes de exemplo para desenvolvimento; a tabela real vem do cadastro de exames da clínica.
CREATE TABLE agenda.procedimento (
  tuss text PRIMARY KEY CHECK (tuss ~ '^[0-9]{8}$'),
  nome text NOT NULL,
  requer_contraste boolean NOT NULL DEFAULT false,
  duracao_min int NOT NULL DEFAULT 30 CHECK (duracao_min BETWEEN 5 AND 240),
  ativo boolean NOT NULL DEFAULT true,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL DEFAULT 'catalogo',
  estacao text, unidade_id uuid REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL DEFAULT 'catalogo'
);
GRANT SELECT ON agenda.procedimento TO vidacor_app;
INSERT INTO agenda.procedimento (tuss, nome, requer_contraste) VALUES
  ('40901114', 'Ecocardiograma transtorácico', false),
  ('40901300', 'Ecodoppler de carótidas', false),
  ('40901220', 'Ecocardiograma com estresse', false),
  ('40901203', 'USG obstétrica', false),
  ('40901122', 'USG de tireoide', false),
  ('40801012', 'Raio X de tórax', false),
  ('41001109', 'TC de abdome com contraste', true),
  ('41001010', 'TC de tórax sem contraste', false);

-- O núcleo (sistema) também atualiza agendamento: expiração da reserva provisória.
DROP POLICY agendamento_unidade ON agenda.agendamento;
CREATE POLICY agendamento_unidade ON agenda.agendamento
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','tecnico','sistema'));
