-- 0013 · Apoio ao laudo: saber a qual paciente um laudo pertence sem ler o conteúdo, para
-- distinguir "sem vínculo" (403 com oferta de acesso de emergência) de "não existe" (FC-ACE-03).
CREATE FUNCTION laudos.paciente_do_laudo(p_laudo uuid) RETURNS uuid LANGUAGE sql STABLE
SECURITY DEFINER SET search_path = pg_catalog, public AS $$
  SELECT l.paciente_id FROM laudos.laudo l WHERE l.id = p_laudo AND l.unidade_id = comum.ctx_unidade()
$$;
GRANT EXECUTE ON FUNCTION laudos.paciente_do_laudo(uuid) TO vidacor_app;
-- O núcleo grava o vínculo do médico atribuído ao criar o laudo (contexto sistema).
ALTER TABLE laudos.versao ALTER COLUMN texto SET DEFAULT '';
