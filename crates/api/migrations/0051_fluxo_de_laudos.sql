-- 0051 · Fluxo de laudos (Fluxo-Laudos.dc.html; docs/04): quadro por etapa, prazo e produtividade
-- para a coordenação (gerência). A gerência não lê laudo (RLS): estas funções devolvem só METADADOS
-- (etapa, prazo, médico, paciente com nome e nascimento, exame), nunca o texto. Prazo padrão de 24 h
-- desde o recebimento das imagens, a confirmar com a clínica (DEC-2026-11-08-96).

CREATE FUNCTION laudos.quadro(p_de timestamptz, p_ate timestamptz)
RETURNS TABLE (
  laudo_id uuid, codigo text, etapa text, paciente_nome text, paciente_nascimento date, exame text,
  modalidade text, medico_id uuid, medico_nome text, recebido_em timestamptz, prazo_em timestamptz,
  assinado_em timestamptz, versao_vigente int, atrasado boolean
)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, laudos, exames, recepcao, agenda, identidade, portal, comum AS $$
BEGIN
  IF NOT comum.ctx_tem_perfil('gerente') THEN
    RAISE EXCEPTION 'quadro de laudos só para a coordenação' USING ERRCODE = '42501';
  END IF;
  RETURN QUERY
  SELECT l.id, l.codigo,
         CASE
           WHEN l.situacao = 'retificacao_em_rascunho' THEN 'em_retificacao'
           WHEN l.situacao = 'rascunho' THEN 'a_laudar'
           WHEN l.conversar_antes AND l.liberado_apos_conversa_em IS NULL THEN 'aguardando_conversa'
           WHEN EXISTS (SELECT 1 FROM portal.publicacao u WHERE u.laudo_id = l.id AND u.vigente) THEN 'entregue'
           ELSE 'assinado'
         END,
         p.nome, p.nascimento, g.exame_nome, s.modalidade, l.medico_atribuido_id, m.nome,
         e.recebido_em, e.recebido_em + interval '24 hours',
         (SELECT v.assinada_em FROM laudos.versao v WHERE v.laudo_id = l.id AND v.situacao = 'assinada'),
         l.versao_vigente,
         l.situacao <> 'assinado' AND now() > e.recebido_em + interval '24 hours'
    FROM laudos.laudo l
    JOIN exames.estudo e ON e.id = l.estudo_id AND e.conciliado
    JOIN recepcao.atendimento t ON t.id = l.atendimento_id
    JOIN agenda.agendamento g ON g.id = t.agendamento_id
    JOIN agenda.sala s ON s.id = t.sala_id
    JOIN identidade.paciente p ON p.id = l.paciente_id
    LEFT JOIN identidade.usuario m ON m.id = l.medico_atribuido_id
   WHERE l.unidade_id = comum.ctx_unidade()
     AND (l.situacao <> 'assinado' OR e.recebido_em >= p_de) AND e.recebido_em < p_ate
   ORDER BY e.recebido_em
   LIMIT 500;
END $$;
REVOKE ALL ON FUNCTION laudos.quadro(timestamptz, timestamptz) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION laudos.quadro(timestamptz, timestamptz) TO vidacor_app;

-- Retificações recentes: laudo, versão, quando e motivo (sem o texto do laudo).
CREATE FUNCTION laudos.retificacoes(p_desde timestamptz)
RETURNS TABLE (laudo_id uuid, codigo text, paciente_nome text, exame text, numero int, assinada_em timestamptz, motivo text, medico_nome text)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, laudos, recepcao, agenda, identidade, comum AS $$
BEGIN
  IF NOT comum.ctx_tem_perfil('gerente') THEN
    RAISE EXCEPTION 'retificações só para a coordenação' USING ERRCODE = '42501';
  END IF;
  RETURN QUERY
  SELECT l.id, l.codigo, p.nome, g.exame_nome, v.numero, v.assinada_em, v.motivo, m.nome
    FROM laudos.versao v
    JOIN laudos.laudo l ON l.id = v.laudo_id
    JOIN recepcao.atendimento t ON t.id = l.atendimento_id
    JOIN agenda.agendamento g ON g.id = t.agendamento_id
    JOIN identidade.paciente p ON p.id = l.paciente_id
    LEFT JOIN identidade.usuario m ON m.id = v.assinada_por
   WHERE l.unidade_id = comum.ctx_unidade() AND v.numero > 1 AND v.assinada_em >= p_desde
   ORDER BY v.assinada_em DESC
   LIMIT 50;
END $$;
REVOKE ALL ON FUNCTION laudos.retificacoes(timestamptz) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION laudos.retificacoes(timestamptz) TO vidacor_app;
