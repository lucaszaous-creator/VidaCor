-- 0081 · Gestão de dose (Gestao-Dose.dc.html; docs/21 § 3, Anvisa RDC 611/2022; DEC-2026-11-09-144 a 149):
-- dose de radiação de cada exame de TC e raio X lida do relatório estruturado de dose do aparelho
-- (RDSR, DICOM SR), comparada ao nível de referência de diagnóstico (NRD) da clínica para o protocolo,
-- com justificativa quando passa do NRD. Revisão humana obrigatória antes de rodar fora de dev (G6).
-- Só acrescenta tabelas e funções; nada é apagado.

-- NRD da clínica por exame (protocolo = procedimento do catálogo), versionado com motivo.
CREATE TABLE exames.nrd (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  exame_tuss text NOT NULL REFERENCES agenda.procedimento(tuss),
  numero int NOT NULL CHECK (numero >= 1),
  -- Grandeza comparada: CTDIvol (mGy) e DLP (mGy·cm) na TC; DAP (Gy·cm²) no raio X.
  grandeza text NOT NULL CHECK (grandeza IN ('ctdivol','dlp','dap')),
  valor double precision NOT NULL CHECK (valor > 0 AND valor <= 100000),
  -- Fator para a dose efetiva estimada (mSv por unidade da grandeza: k do DLP ou do DAP). Opcional.
  fator_dose_efetiva double precision CHECK (fator_dose_efetiva > 0 AND fator_dose_efetiva <= 10),
  vigente boolean NOT NULL DEFAULT true,
  motivo text CHECK (length(trim(motivo)) BETWEEN 5 AND 300),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  UNIQUE (exame_tuss, numero),
  CHECK (numero = 1 OR motivo IS NOT NULL)
);
CREATE UNIQUE INDEX nrd_um_vigente ON exames.nrd (exame_tuss) WHERE vigente;
CREATE TRIGGER nrd_versionado BEFORE UPDATE OR DELETE ON exames.nrd
  FOR EACH ROW EXECUTE FUNCTION documentos.proteger_versionado();

-- Dose lida do RDSR: uma por estudo (repetir a leitura não duplica, G3).
CREATE TABLE exames.dose (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  estudo_id uuid NOT NULL UNIQUE REFERENCES exames.estudo(id),
  atendimento_id uuid NOT NULL REFERENCES recepcao.atendimento(id),
  paciente_id uuid NOT NULL REFERENCES identidade.paciente(id),
  sala_id uuid NOT NULL REFERENCES agenda.sala(id),
  aparelho text,
  modalidade text NOT NULL CHECK (modalidade ~ '^[A-Z]{2,4}$'),
  exame_tuss text NOT NULL,
  ctdivol_mgy double precision CHECK (ctdivol_mgy >= 0),
  dlp_mgycm double precision CHECK (dlp_mgycm >= 0),
  dap_gycm2 double precision CHECK (dap_gycm2 >= 0),
  -- Origem do valor: o objeto RDSR no PACS e o SHA-256 das tags lidas (lastro).
  rdsr_sop_uid text NOT NULL CHECK (rdsr_sop_uid ~ '^[0-9.]{1,64}$'),
  rdsr_sha256 text NOT NULL CHECK (rdsr_sha256 ~ '^[0-9a-f]{64}$'),
  -- Comparação com o NRD vigente no momento da leitura (versão aplicada fica gravada).
  nrd_id uuid REFERENCES exames.nrd(id),
  valor_comparado double precision,
  acima_do_nrd boolean NOT NULL DEFAULT false,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  CHECK (ctdivol_mgy IS NOT NULL OR dlp_mgycm IS NOT NULL OR dap_gycm2 IS NOT NULL),
  CHECK (NOT acima_do_nrd OR nrd_id IS NOT NULL)
);
CREATE INDEX dose_paciente_idx ON exames.dose (paciente_id, criado_em);
CREATE INDEX dose_periodo_idx ON exames.dose (unidade_id, criado_em);

-- Tentativas de leitura do RDSR por estudo (o SR pode chegar depois das imagens). Para de tentar
-- 24 h depois da chegada do estudo.
CREATE TABLE exames.dose_tentativa (
  estudo_id uuid PRIMARY KEY REFERENCES exames.estudo(id),
  tentativas int NOT NULL DEFAULT 1 CHECK (tentativas >= 1),
  ultima_em timestamptz NOT NULL DEFAULT now(),
  resultado text NOT NULL CHECK (resultado IN ('sem_rdsr','rdsr_sem_dose','erro')),
  -- Metadados de lastro (docs/21; conferidos em agenda_e_banco.rs). Quem grava é o núcleo (criado_por nulo).
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL DEFAULT 'sistema',
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL
);

-- Justificativa de dose acima do NRD: uma por dose, imutável (correção seria nova linha em outra entrega).
CREATE TABLE exames.dose_justificativa (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  dose_id uuid NOT NULL UNIQUE REFERENCES exames.dose(id),
  motivo text NOT NULL CHECK (motivo IN ('imc_alto','repeticao_movimento','fase_extra','implante_metalico','outro')),
  detalhe text CHECK (length(trim(detalhe)) BETWEEN 5 AND 500),
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid NOT NULL REFERENCES identidade.usuario(id),
  origem text NOT NULL, estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  versao int NOT NULL DEFAULT 1, correlacao text NOT NULL,
  CHECK (motivo <> 'outro' OR detalhe IS NOT NULL)
);

CREATE FUNCTION exames.dose_imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION '% é registro de lastro: não muda nem é apagado', TG_TABLE_NAME;
END$$;
CREATE TRIGGER dose_imutavel BEFORE UPDATE OR DELETE ON exames.dose
  FOR EACH ROW EXECUTE FUNCTION exames.dose_imutavel();
CREATE TRIGGER justificativa_imutavel BEFORE UPDATE OR DELETE ON exames.dose_justificativa
  FOR EACH ROW EXECUTE FUNCTION exames.dose_imutavel();

-- RLS. Dose por paciente é dado de saúde (N3): o médico lê com vínculo de cuidado (mesma regra do
-- laudo); o núcleo grava. A gerência vê só agregados, pela função abaixo.
ALTER TABLE exames.dose ENABLE ROW LEVEL SECURITY;
ALTER TABLE exames.dose FORCE ROW LEVEL SECURITY;
CREATE POLICY dose_leitura ON exames.dose FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND (comum.ctx_tem_perfil('sistema')
    OR (comum.ctx_tem_perfil('medico') AND cuidado.pode_ler(paciente_id))));
CREATE POLICY dose_escrita ON exames.dose FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema'));

ALTER TABLE exames.dose_justificativa ENABLE ROW LEVEL SECURITY;
ALTER TABLE exames.dose_justificativa FORCE ROW LEVEL SECURITY;
CREATE POLICY justificativa_leitura ON exames.dose_justificativa FOR SELECT
  USING (unidade_id = comum.ctx_unidade() AND (comum.ctx_tem_perfil('sistema')
    OR (comum.ctx_tem_perfil('medico') AND EXISTS (SELECT 1 FROM exames.dose d WHERE d.id = dose_id))));
CREATE POLICY justificativa_escrita ON exames.dose_justificativa FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('medico')
    AND criado_por = NULLIF(current_setting('app.usuario_id', true), '')::uuid
    AND EXISTS (SELECT 1 FROM exames.dose d WHERE d.id = dose_id));

ALTER TABLE exames.dose_tentativa ENABLE ROW LEVEL SECURITY;
ALTER TABLE exames.dose_tentativa FORCE ROW LEVEL SECURITY;
CREATE POLICY tentativa_sistema ON exames.dose_tentativa
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('sistema'));

ALTER TABLE exames.nrd ENABLE ROW LEVEL SECURITY;
ALTER TABLE exames.nrd FORCE ROW LEVEL SECURITY;
-- NRD não identifica paciente: médico, gerência e o núcleo leem; só a gerência publica.
CREATE POLICY nrd_leitura ON exames.nrd FOR SELECT
  USING (comum.ctx_tem_perfil('medico','gerente','sistema'));
CREATE POLICY nrd_escrita ON exames.nrd FOR INSERT
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('gerente'));
CREATE POLICY nrd_atualizacao ON exames.nrd FOR UPDATE
  USING (comum.ctx_tem_perfil('gerente')) WITH CHECK (comum.ctx_tem_perfil('gerente'));

GRANT SELECT, INSERT, UPDATE ON exames.nrd TO vidacor_app;
GRANT SELECT, INSERT ON exames.dose, exames.dose_justificativa TO vidacor_app;
GRANT SELECT, INSERT, UPDATE ON exames.dose_tentativa TO vidacor_app;

-- Estudos conciliados de modalidade com radiação ainda sem dose, para o trabalho de leitura (núcleo).
CREATE FUNCTION exames.estudos_sem_dose(p_limite int)
RETURNS TABLE (estudo_id uuid, orthanc_id text, atendimento_id uuid, paciente_id uuid, sala_id uuid,
               modalidade text, aparelho text, exame_tuss text)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, exames, recepcao, agenda, comum AS $$
BEGIN
  IF NOT comum.ctx_tem_perfil('sistema') THEN
    RAISE EXCEPTION 'leitura de dose só pelo núcleo' USING ERRCODE = '42501';
  END IF;
  RETURN QUERY
  SELECT e.id, e.orthanc_id, t.id, t.paciente_id, t.sala_id, ap.modalidade, ap.nome, g.exame_tuss
    FROM exames.estudo e
    JOIN recepcao.atendimento t ON t.id = e.atendimento_id
    JOIN agenda.agendamento g ON g.id = t.agendamento_id
    JOIN LATERAL (SELECT a.modalidade, a.nome FROM agenda.aparelho a
                   WHERE a.sala_id = t.sala_id AND a.ativo ORDER BY a.codigo LIMIT 1) ap ON true
   WHERE e.unidade_id = comum.ctx_unidade() AND e.conciliado
     AND ap.modalidade IN ('CT','DX','CR','MG','XA','RF')
     AND e.recebido_em > now() - interval '24 hours'
     AND NOT EXISTS (SELECT 1 FROM exames.dose d WHERE d.estudo_id = e.id)
     AND NOT EXISTS (SELECT 1 FROM exames.dose_tentativa x
                      WHERE x.estudo_id = e.id AND x.ultima_em > now() - make_interval(mins => least(60, 2 * x.tentativas)))
   ORDER BY e.recebido_em
   LIMIT p_limite;
END $$;
REVOKE ALL ON FUNCTION exames.estudos_sem_dose(int) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION exames.estudos_sem_dose(int) TO vidacor_app;

-- Agregado por protocolo (sem paciente) para a gerência e o médico: exames, mediana e quantos acima.
CREATE FUNCTION exames.dose_por_protocolo(p_de timestamptz, p_ate timestamptz)
RETURNS TABLE (exame_tuss text, exame text, aparelho text, grandeza text, exames bigint,
               mediana double precision, nrd double precision, acima bigint)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, exames, agenda, comum AS $$
BEGIN
  IF NOT comum.ctx_tem_perfil('gerente','medico') THEN
    RAISE EXCEPTION 'agregado de dose só para a coordenação e os médicos' USING ERRCODE = '42501';
  END IF;
  RETURN QUERY
  SELECT d.exame_tuss, coalesce(p.nome, d.exame_tuss), coalesce(d.aparelho, ''), coalesce(n.grandeza, ''),
         count(*),
         percentile_cont(0.5) WITHIN GROUP (ORDER BY d.valor_comparado),
         max(n.valor),
         count(*) FILTER (WHERE d.acima_do_nrd)
    FROM exames.dose d
    LEFT JOIN agenda.procedimento p ON p.tuss = d.exame_tuss
    LEFT JOIN exames.nrd n ON n.id = d.nrd_id
   WHERE d.unidade_id = comum.ctx_unidade() AND d.criado_em >= p_de AND d.criado_em < p_ate
   GROUP BY d.exame_tuss, p.nome, d.aparelho, n.grandeza
   ORDER BY count(*) DESC
   LIMIT 100;
END $$;
REVOKE ALL ON FUNCTION exames.dose_por_protocolo(timestamptz, timestamptz) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION exames.dose_por_protocolo(timestamptz, timestamptz) TO vidacor_app;
