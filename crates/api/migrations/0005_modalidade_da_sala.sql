-- 0005 · Modalidade DICOM da sala (exigida pela worklist: ScheduledProcedureStepSequence.Modality).
ALTER TABLE agenda.sala ADD COLUMN modalidade text NOT NULL DEFAULT 'OT'
  CHECK (modalidade ~ '^[A-Z]{2,4}$');
COMMENT ON COLUMN agenda.sala.modalidade IS 'Código de modalidade DICOM (US, DX, CT, ECG, OT)';
