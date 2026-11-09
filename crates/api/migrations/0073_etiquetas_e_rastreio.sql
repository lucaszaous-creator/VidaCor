-- 0073 · Etiquetas e rastreio (Etiquetas.dc.html, docs/20): painel do dia, histórico de cada etiqueta,
-- invalidação com motivo e resolução da divergência de pulseira bipada na sala errada.
-- A divergência só passa de aberta para resolvida, uma vez; nada se apaga (docs/21). A resolução fica em
-- `dados` (resolucao, resolvida_por, resolvida_em), como a contingência já faz (0016).

CREATE FUNCTION recepcao.divergencia_sem_alteracao() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN RAISE EXCEPTION 'divergência não se apaga' USING ERRCODE = '42501'; END IF;
  IF OLD.situacao <> 'aberta'
     OR (NEW.id, NEW.codigo, NEW.tipo, NEW.atendimento_id, NEW.sala_id, NEW.criado_em, NEW.criado_por, NEW.unidade_id)
        IS DISTINCT FROM (OLD.id, OLD.codigo, OLD.tipo, OLD.atendimento_id, OLD.sala_id, OLD.criado_em, OLD.criado_por, OLD.unidade_id) THEN
    RAISE EXCEPTION 'divergência só muda de aberta para resolvida, uma vez' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER divergencia_imutavel BEFORE UPDATE OR DELETE ON recepcao.divergencia
  FOR EACH ROW EXECUTE FUNCTION recepcao.divergencia_sem_alteracao();

-- A etiqueta invalidada não volta a valer nem muda de dono; nada se apaga (docs/20).
CREATE FUNCTION recepcao.etiqueta_sem_volta() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN RAISE EXCEPTION 'etiqueta não se apaga' USING ERRCODE = '42501'; END IF;
  IF (NEW.id, NEW.codigo, NEW.tipo, NEW.atendimento_id, NEW.paciente_id, NEW.criado_em, NEW.unidade_id)
       IS DISTINCT FROM (OLD.id, OLD.codigo, OLD.tipo, OLD.atendimento_id, OLD.paciente_id, OLD.criado_em, OLD.unidade_id)
     OR (OLD.situacao = 'invalidada' AND NEW.situacao <> 'invalidada') THEN
    RAISE EXCEPTION 'etiqueta invalidada não volta a valer e não muda de paciente' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER etiqueta_sem_volta BEFORE UPDATE OR DELETE ON recepcao.etiqueta
  FOR EACH ROW EXECUTE FUNCTION recepcao.etiqueta_sem_volta();

-- Histórico da etiqueta: eventos do lastro que citam o código (bipe, divergência), além do registro dela.
CREATE INDEX evento_etiqueta_idx ON lastro.evento ((dados->>'etiqueta')) WHERE dados ? 'etiqueta';
CREATE INDEX etiqueta_dia_idx ON recepcao.etiqueta (unidade_id, criado_em);
