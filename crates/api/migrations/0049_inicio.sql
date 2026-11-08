-- 0049 · Início (Main.dc.html): contagem de laudos a liberar sem expor linha de laudo (a recepção e a
-- gerência não leem laudos; só o médico com vínculo). Só a unidade do contexto (DEC-2026-11-08-90).
CREATE FUNCTION laudos.contagem_a_liberar()
RETURNS TABLE (a_liberar bigint, em_retificacao bigint)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, laudos, exames, comum AS $$
  SELECT count(*) FILTER (WHERE l.situacao = 'rascunho'),
         count(*) FILTER (WHERE l.situacao = 'retificacao_em_rascunho')
    FROM laudos.laudo l JOIN exames.estudo e ON e.id = l.estudo_id AND e.conciliado
   WHERE l.unidade_id = comum.ctx_unidade()
$$;
REVOKE ALL ON FUNCTION laudos.contagem_a_liberar() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION laudos.contagem_a_liberar() TO vidacor_app;
