-- 0004 · Cursor de leitura das mudanças do Orthanc (estudos recebidos), um por unidade.
CREATE TABLE integracao.cursor_orthanc (
  unidade_id uuid PRIMARY KEY REFERENCES identidade.unidade(id),
  ultima_mudanca bigint NOT NULL DEFAULT 0 CHECK (ultima_mudanca >= 0),
  atualizado_em timestamptz NOT NULL DEFAULT now()
);
GRANT SELECT, INSERT, UPDATE ON integracao.cursor_orthanc TO vidacor_app;
