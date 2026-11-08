-- 0006 · Recepção, etiquetas e sala (docs/27 FC-CHK, FC-ETQ, FC-PUL; docs/20; docs/30 § 1)

-- Reautenticação (SEC-AUT-5): instante do último login com senha e 2FA nesta sessão.
ALTER TABLE identidade.sessao ADD COLUMN reautenticado_em timestamptz NOT NULL DEFAULT now();
-- Pedido de reautenticação: liga o retorno do provedor à sessão que pediu e para onde voltar.
ALTER TABLE identidade.login_pendente
  ADD COLUMN sessao_id uuid REFERENCES identidade.sessao(id),
  ADD COLUMN voltar text CHECK (voltar IS NULL OR (voltar ~ '^/[A-Za-z0-9/_?=&.%-]*$' AND voltar !~ '^//'));

-- Sexo do cadastro: usado na TFG (CKD-EPI) e em exames por sexo.
ALTER TABLE identidade.paciente ADD COLUMN sexo text CHECK (sexo IN ('F', 'M'));

-- Histórico de versões do cadastro: correção gera versão com motivo, nada é sobrescrito sem rastro.
CREATE TABLE identidade.paciente_versao (
  id uuid PRIMARY KEY, paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  versao int NOT NULL, nome text NOT NULL, nascimento date NOT NULL, sexo text,
  motivo text NOT NULL CHECK (length(trim(motivo)) >= 3),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), correlacao text NOT NULL,
  UNIQUE (paciente_id, versao)
);
ALTER TABLE identidade.paciente_versao ENABLE ROW LEVEL SECURITY;
ALTER TABLE identidade.paciente_versao FORCE ROW LEVEL SECURITY;
CREATE POLICY paciente_versao_unidade ON identidade.paciente_versao
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente'));
GRANT SELECT, INSERT ON identidade.paciente_versao TO vidacor_app;

-- Lotação do técnico por sala (FC-PUL-05).
CREATE TABLE identidade.lotacao (
  usuario_id uuid NOT NULL REFERENCES identidade.usuario(id),
  sala_id uuid NOT NULL REFERENCES agenda.sala(id),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  PRIMARY KEY (usuario_id, sala_id)
);
GRANT SELECT ON identidade.lotacao TO vidacor_app;

-- Aparelhos cadastrados por sala (SEC-MSG-7). A worklist vai para o AE Title do aparelho da sala.
CREATE TABLE agenda.aparelho (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE, nome text NOT NULL,
  ae_title text NOT NULL UNIQUE CHECK (ae_title ~ '^[A-Z0-9_]{1,16}$'),
  modalidade text NOT NULL CHECK (modalidade ~ '^[A-Z]{2,4}$'),
  sala_id uuid NOT NULL REFERENCES agenda.sala(id),
  ativo boolean NOT NULL DEFAULT true,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
CREATE INDEX aparelho_sala_idx ON agenda.aparelho (sala_id);
GRANT SELECT ON agenda.aparelho TO vidacor_app;

ALTER TABLE agenda.agendamento
  ADD COLUMN requer_contraste boolean NOT NULL DEFAULT false,
  ADD COLUMN medico_id uuid REFERENCES identidade.usuario(id);

-- Atendimento: situação "aguardando sala" (FC-CHK-01), senha e conferência do documento.
ALTER TABLE recepcao.atendimento DROP CONSTRAINT atendimento_situacao_check;
UPDATE recepcao.atendimento SET situacao = 'aguardando_sala' WHERE situacao = 'aguardando_exame';
ALTER TABLE recepcao.atendimento
  ADD CONSTRAINT atendimento_situacao_check
    CHECK (situacao IN ('aguardando_sala','em_exame','exame_concluido','cancelado')),
  ADD COLUMN senha text,
  ADD COLUMN documento_conferido boolean NOT NULL DEFAULT false;

-- Senha do dia por unidade: contador atômico (G2), recomeça a cada dia.
CREATE TABLE recepcao.senha_dia (
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  dia date NOT NULL,
  valor int NOT NULL DEFAULT 0 CHECK (valor >= 0),
  PRIMARY KEY (unidade_id, dia)
);
GRANT SELECT, INSERT, UPDATE ON recepcao.senha_dia TO vidacor_app;
CREATE FUNCTION recepcao.proxima_senha(p_unidade uuid, p_dia date) RETURNS int LANGUAGE sql AS $$
  INSERT INTO recepcao.senha_dia AS s (unidade_id, dia, valor) VALUES (p_unidade, p_dia, 1)
  ON CONFLICT (unidade_id, dia) DO UPDATE SET valor = s.valor + 1
  RETURNING valor
$$;
GRANT EXECUTE ON FUNCTION recepcao.proxima_senha(uuid, date) TO vidacor_app;

-- Etiquetas (docs/20): código único, reimpressão com motivo invalida a anterior. Conteúdo impresso
-- só nome, nascimento e códigos (nunca diagnóstico): a tabela não guarda texto livre clínico.
CREATE TABLE recepcao.etiqueta (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  tipo text NOT NULL CHECK (tipo IN ('pulseira')),
  atendimento_id uuid NOT NULL REFERENCES recepcao.atendimento(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  situacao text NOT NULL CHECK (situacao IN ('valida','invalidada')),
  motivo_invalidacao text,
  invalidada_por uuid, invalidada_em timestamptz,
  substituida_por text,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK (situacao = 'valida' OR motivo_invalidacao IS NOT NULL)
);
-- No máximo uma pulseira válida por atendimento (G2).
CREATE UNIQUE INDEX etiqueta_uma_valida ON recepcao.etiqueta (atendimento_id, tipo) WHERE situacao = 'valida';
CREATE INDEX etiqueta_paciente_idx ON recepcao.etiqueta (paciente_id);
ALTER TABLE recepcao.etiqueta ENABLE ROW LEVEL SECURITY;
ALTER TABLE recepcao.etiqueta FORCE ROW LEVEL SECURITY;
CREATE POLICY etiqueta_unidade ON recepcao.etiqueta
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('recepcao','gerente','tecnico'));
GRANT SELECT, INSERT, UPDATE ON recepcao.etiqueta TO vidacor_app;

-- Divergências (pulseira na sala errada, identificação, preço): abertas com dono e prazo.
CREATE TABLE recepcao.divergencia (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  tipo text NOT NULL CHECK (tipo IN ('pulseira_sala_errada','identificacao','preco')),
  atendimento_id uuid REFERENCES recepcao.atendimento(id),
  sala_id uuid REFERENCES agenda.sala(id),
  dados jsonb NOT NULL,
  situacao text NOT NULL DEFAULT 'aberta' CHECK (situacao IN ('aberta','resolvida')),
  prazo timestamptz NOT NULL DEFAULT now() + interval '1 day',
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);
ALTER TABLE recepcao.divergencia ENABLE ROW LEVEL SECURITY;
ALTER TABLE recepcao.divergencia FORCE ROW LEVEL SECURITY;
CREATE POLICY divergencia_unidade ON recepcao.divergencia
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_atendimento())
  WITH CHECK (unidade_id = comum.ctx_unidade());
GRANT SELECT, INSERT, UPDATE ON recepcao.divergencia TO vidacor_app;

-- Outbox com referência de negócio: um item por (tipo, referência), repetir não duplica (FC-PUL-04).
ALTER TABLE integracao.outbox ADD COLUMN referencia uuid;
CREATE UNIQUE INDEX outbox_tipo_referencia ON integracao.outbox (tipo, referencia) WHERE referencia IS NOT NULL;
