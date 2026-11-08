-- 0014 · O laudo informa se já está no portal sem expor a publicação à equipe (a tabela do portal
-- só é lida por paciente ou sistema). Devolve uma linha se houver publicação vigente.
CREATE FUNCTION portal.publicacao_existe(p_laudo uuid) RETURNS TABLE (vigente boolean) LANGUAGE sql STABLE
SECURITY DEFINER SET search_path = pg_catalog, public AS $$
  SELECT true FROM portal.publicacao p WHERE p.laudo_id = p_laudo AND p.vigente LIMIT 1
$$;
GRANT EXECUTE ON FUNCTION portal.publicacao_existe(uuid) TO vidacor_app;
-- Auditoria do portal guarda quem é o paciente e o que foi pedido (FC-ACE-01).
