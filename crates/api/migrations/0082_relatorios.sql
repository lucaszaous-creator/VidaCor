-- 0082 · Relatórios e exportação (Relatorios.dc.html; DEC-2026-11-09-150 a 154): catálogo fechado de
-- relatórios calculados no núcleo, prévia e exportação em CSV com reautenticação (SEC-AUT-5). Cada
-- exportação fica registrada com o SHA-256 do arquivo, quem, quando, filtro e se havia dado de saúde
-- (sempre removido: paciente vira código PAC e exame vira código TUSS). Revisão humana obrigatória antes
-- de rodar fora de dev (G6). Só acrescenta; nada é apagado.

CREATE SCHEMA relatorios;
GRANT USAGE ON SCHEMA relatorios TO vidacor_app;

CREATE TABLE relatorios.exportacao (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  relatorio text NOT NULL CHECK (relatorio IN
    ('producao_medico','producao_convenio','producao_sala','faltas','glosas','financeiro','estoque')),
  formato text NOT NULL CHECK (formato IN ('csv')),
  de date NOT NULL, ate date NOT NULL CHECK (ate >= de AND ate - de <= 366),
  -- sem_dado_de_saude: o relatório não tem; pseudonimizado: tinha e saiu como código.
  conteudo text NOT NULL CHECK (conteudo IN ('sem_dado_de_saude','pseudonimizado')),
  linhas int NOT NULL CHECK (linhas >= 0),
  sha256 text NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL
);
CREATE INDEX exportacao_quem_idx ON relatorios.exportacao (unidade_id, criado_em);

CREATE FUNCTION relatorios.imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'registro de exportação não muda nem é apagado';
END$$;
CREATE TRIGGER exportacao_imutavel BEFORE UPDATE OR DELETE ON relatorios.exportacao
  FOR EACH ROW EXECUTE FUNCTION relatorios.imutavel();

ALTER TABLE relatorios.exportacao ENABLE ROW LEVEL SECURITY;
ALTER TABLE relatorios.exportacao FORCE ROW LEVEL SECURITY;
-- Cada um vê as próprias exportações; a gerência e o auditor veem as da unidade.
CREATE POLICY exportacao_leitura ON relatorios.exportacao FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND (comum.ctx_tem_perfil('gerente','auditor')
    OR criado_por = NULLIF(current_setting('app.usuario_id', true), '')::uuid));
CREATE POLICY exportacao_registro ON relatorios.exportacao FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade()
    AND criado_por = NULLIF(current_setting('app.usuario_id', true), '')::uuid);
GRANT SELECT, INSERT ON relatorios.exportacao TO vidacor_app;

-- Produção por médico: laudos assinados (versão assinada) no período, sem texto de laudo. A gerência não
-- lê laudo pelo RLS; esta função devolve só contagens por médico e o exame mais frequente.
CREATE FUNCTION relatorios.producao_por_medico(p_de timestamptz, p_ate timestamptz)
RETURNS TABLE (medico text, principal_exame text, laudos bigint)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, laudos, recepcao, agenda, identidade, comum AS $$
BEGIN
  IF NOT comum.ctx_tem_perfil('gerente') THEN
    RAISE EXCEPTION 'produção por médico só para a coordenação' USING ERRCODE = '42501';
  END IF;
  RETURN QUERY
  WITH assinadas AS (
    SELECT v.assinada_por, g.exame_nome
      FROM laudos.versao v
      JOIN laudos.laudo l ON l.id = v.laudo_id
      JOIN recepcao.atendimento t ON t.id = l.atendimento_id
      JOIN agenda.agendamento g ON g.id = t.agendamento_id
     WHERE l.unidade_id = comum.ctx_unidade() AND v.situacao = 'assinada'
       AND v.assinada_em >= p_de AND v.assinada_em < p_ate
  )
  SELECT u.nome,
         (SELECT a2.exame_nome FROM assinadas a2 WHERE a2.assinada_por = a.assinada_por
           GROUP BY a2.exame_nome ORDER BY count(*) DESC, a2.exame_nome LIMIT 1),
         count(*)
    FROM assinadas a JOIN identidade.usuario u ON u.id = a.assinada_por
   GROUP BY a.assinada_por, u.nome
   ORDER BY count(*) DESC, u.nome
   LIMIT 200;
END $$;
REVOKE ALL ON FUNCTION relatorios.producao_por_medico(timestamptz, timestamptz) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION relatorios.producao_por_medico(timestamptz, timestamptz) TO vidacor_app;
