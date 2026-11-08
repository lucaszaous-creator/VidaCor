-- 0048 · Aparelhos e integração DICOM (Aparelhos.dc.html; docs/18: a TI administra aparelhos e
-- integrações, sem dado clínico). Cadastro, desativação e reativação com motivo; cada mudança fica em
-- histórico imutável (configuração versionada, docs/21). Aparelho nunca é apagado.

CREATE TABLE agenda.aparelho_mudanca (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  aparelho_id uuid NOT NULL REFERENCES agenda.aparelho(id),
  tipo text NOT NULL CHECK (tipo IN ('cadastrar','desativar','reativar')),
  motivo text NOT NULL CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  versao_do_aparelho int NOT NULL,
  dados jsonb NOT NULL,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  UNIQUE (aparelho_id, versao_do_aparelho)
);
CREATE FUNCTION agenda.aparelho_mudanca_imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'histórico de aparelho é imutável' USING ERRCODE = '42501'; END $$;
CREATE TRIGGER aparelho_mudanca_sem_alteracao BEFORE UPDATE OR DELETE ON agenda.aparelho_mudanca
  FOR EACH ROW EXECUTE FUNCTION agenda.aparelho_mudanca_imutavel();
ALTER TABLE agenda.aparelho_mudanca ENABLE ROW LEVEL SECURITY;
ALTER TABLE agenda.aparelho_mudanca FORCE ROW LEVEL SECURITY;
CREATE POLICY aparelho_mudanca_unidade ON agenda.aparelho_mudanca
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('ti','gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('ti','gerente'));
GRANT SELECT, INSERT ON agenda.aparelho_mudanca TO vidacor_app;

-- O cadastro de aparelhos continua legível por quem usa a sala (bipe, worklist), mas só TI e
-- gerência da MESMA unidade escrevem nele; nunca se apaga (barreira por gatilho, como em 0043).
CREATE FUNCTION agenda.aparelho_escrita_permitida() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN RAISE EXCEPTION 'aparelho não se apaga; desative com motivo' USING ERRCODE = '42501'; END IF;
  IF current_user = 'vidacor_app' AND NOT (comum.ctx_tem_perfil('ti','gerente') AND NEW.unidade_id = comum.ctx_unidade()) THEN
    RAISE EXCEPTION 'escrita em agenda.aparelho negada' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER aparelho_escrita BEFORE INSERT OR UPDATE OR DELETE ON agenda.aparelho
  FOR EACH ROW EXECUTE FUNCTION agenda.aparelho_escrita_permitida();
GRANT INSERT, UPDATE (ativo, versao) ON agenda.aparelho TO vidacor_app;

-- Contagens do dia para o painel de aparelhos, sem nenhuma linha de estudo: a TI não lê
-- exames.estudo (dado clínico). Só a unidade do contexto.
CREATE FUNCTION exames.contagem_do_dia(p_inicio timestamptz, p_fim timestamptz)
RETURNS TABLE (recebidos bigint, sem_paciente bigint)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, exames, comum AS $$
  SELECT count(*) FILTER (WHERE recebido_em >= p_inicio AND recebido_em < p_fim),
         count(*) FILTER (WHERE NOT conciliado)
    FROM exames.estudo
   WHERE unidade_id = comum.ctx_unidade()
$$;
REVOKE ALL ON FUNCTION exames.contagem_do_dia(timestamptz, timestamptz) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION exames.contagem_do_dia(timestamptz, timestamptz) TO vidacor_app;
