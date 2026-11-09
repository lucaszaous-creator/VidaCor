-- 0066 · Check-up e pacotes (Checkup-Pacotes.dc.html; docs/04 Agenda; DEC-2026-11-09-115 a 117).
-- Revisão humana obrigatória antes de rodar fora de dev (G6).
--
-- Reaproveita `financeiro.pacote` (0047), que o orçamento já usa: cada linha é uma VERSÃO da regra
-- (código + regra_versao), com os exames e o preço do pacote. Esta migration só acrescenta: colunas
-- anuláveis (para quem é, motivo), a ordem e o preparo de cada exame no roteiro, a restrição de uma
-- versão por dia de início, gatilhos de proteção e a escrita pela gerência. Nada é apagado. Antes de
-- rodar em banco com dados, conferir (se houver linhas, a migration falha sem mudar nada):
--   SELECT unidade_id, codigo, vigente_desde, count(*) FROM financeiro.pacote GROUP BY 1, 2, 3 HAVING count(*) > 1;

ALTER TABLE financeiro.pacote
  ADD COLUMN para_quem text CHECK (length(trim(para_quem)) BETWEEN 3 AND 200),
  ADD COLUMN motivo text CHECK (length(trim(motivo)) BETWEEN 5 AND 300);
-- Duas versões do mesmo pacote não começam no mesmo dia (G2; a de mesma versão já é única).
ALTER TABLE financeiro.pacote
  ADD CONSTRAINT pacote_uma_por_vigencia UNIQUE (unidade_id, codigo, vigente_desde);

-- Ordem do roteiro e preparo de cada exame da versão (o roteiro do dia parte daqui).
CREATE TABLE financeiro.pacote_item (
  pacote_id uuid NOT NULL REFERENCES financeiro.pacote(id),
  ordem int NOT NULL CHECK (ordem BETWEEN 1 AND 10),
  tuss text NOT NULL REFERENCES agenda.procedimento(tuss),
  preparo text NOT NULL CHECK (length(trim(preparo)) BETWEEN 3 AND 160),
  -- Por que este exame vem nesta posição (mostrado ao paciente e à recepção).
  porque text NOT NULL CHECK (length(trim(porque)) BETWEEN 3 AND 200),
  unidade_id uuid NOT NULL REFERENCES identidade.unidade(id),
  PRIMARY KEY (pacote_id, ordem),
  UNIQUE (pacote_id, tuss)
);

-- Versão de pacote não se altera nem se apaga: a correção é uma versão nova com motivo.
CREATE FUNCTION financeiro.pacote_imutavel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION '% é versionado: a correção é uma versão nova do pacote', TG_TABLE_NAME USING ERRCODE = '42501';
END $$;
CREATE TRIGGER pacote_sem_alteracao BEFORE UPDATE OR DELETE ON financeiro.pacote
  FOR EACH ROW EXECUTE FUNCTION financeiro.pacote_imutavel();
CREATE TRIGGER pacote_item_sem_alteracao BEFORE UPDATE OR DELETE ON financeiro.pacote_item
  FOR EACH ROW EXECUTE FUNCTION financeiro.pacote_imutavel();

-- Pelo papel da aplicação: a versão é da gerência da própria unidade, com motivo; os exames só entram
-- junto com a versão, na mesma transação, e batem com a lista `tuss` da versão.
CREATE FUNCTION financeiro.pacote_escrita_permitida() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF current_user = 'vidacor_app' AND NOT (comum.ctx_tem_perfil('gerente')
       AND NEW.unidade_id = comum.ctx_unidade() AND NEW.motivo IS NOT NULL AND NEW.para_quem IS NOT NULL) THEN
    RAISE EXCEPTION 'escrita em financeiro.pacote negada' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER pacote_escrita BEFORE INSERT ON financeiro.pacote
  FOR EACH ROW EXECUTE FUNCTION financeiro.pacote_escrita_permitida();

CREATE FUNCTION financeiro.pacote_item_escrita_permitida() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF current_user = 'vidacor_app' AND NOT (comum.ctx_tem_perfil('gerente') AND EXISTS (
       SELECT 1 FROM financeiro.pacote p
        WHERE p.id = NEW.pacote_id AND p.unidade_id = comum.ctx_unidade() AND p.unidade_id = NEW.unidade_id
          AND p.criado_em = now() AND NEW.tuss = ANY (p.tuss))) THEN
    RAISE EXCEPTION 'escrita em financeiro.pacote_item negada' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER pacote_item_escrita BEFORE INSERT ON financeiro.pacote_item
  FOR EACH ROW EXECUTE FUNCTION financeiro.pacote_item_escrita_permitida();

GRANT SELECT, INSERT ON financeiro.pacote_item TO vidacor_app;
GRANT INSERT ON financeiro.pacote TO vidacor_app;
