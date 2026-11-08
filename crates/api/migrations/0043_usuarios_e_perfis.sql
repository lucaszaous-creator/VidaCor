-- 0043 · Usuários e perfis (Usuarios-Perfis.dc.html; docs/18 § 6: "quem cria usuário com perfil
-- clínico precisa da aprovação de uma segunda pessoa"; "a TI não tem dado clínico"; mudar permissão
-- pede reautenticação). Toda mudança é um pedido registrado; nada é apagado (desativar, não excluir).

CREATE TABLE identidade.pedido_perfil (
  id uuid PRIMARY KEY, codigo text NOT NULL UNIQUE,
  usuario_id uuid NOT NULL REFERENCES identidade.usuario(id),
  tipo text NOT NULL CHECK (tipo IN ('criar','conceder','retirar','desativar','reativar')),
  perfil text CHECK (perfil IS NULL OR perfil ~ '^[a-z_]{2,30}$'),
  motivo text NOT NULL CHECK (length(trim(motivo)) >= 5),
  situacao text NOT NULL CHECK (situacao IN ('aguardando','aplicado','recusado')),
  pedido_por uuid NOT NULL REFERENCES identidade.usuario(id),
  decidido_por uuid REFERENCES identidade.usuario(id),
  decidido_em timestamptz,
  criado_em timestamptz NOT NULL DEFAULT now(), criado_por uuid, origem text NOT NULL,
  estacao text, unidade_id uuid NOT NULL REFERENCES identidade.unidade(id), versao int NOT NULL DEFAULT 1,
  correlacao text NOT NULL,
  -- Segunda pessoa (docs/18 § 6): quem pede não decide o próprio pedido que precisa de conferência.
  CHECK (decidido_por IS NULL OR situacao = 'aplicado' AND decidido_por = pedido_por AND tipo IN ('retirar','desativar','conceder')
         OR decidido_por <> pedido_por)
);
-- Um pedido aguardando por usuário (G2).
CREATE UNIQUE INDEX pedido_perfil_um_aguardando ON identidade.pedido_perfil (usuario_id) WHERE situacao = 'aguardando';
ALTER TABLE identidade.pedido_perfil ENABLE ROW LEVEL SECURITY;
ALTER TABLE identidade.pedido_perfil FORCE ROW LEVEL SECURITY;
CREATE POLICY pedido_perfil_unidade ON identidade.pedido_perfil
  USING (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('ti','gerente'))
  WITH CHECK (unidade_id = comum.ctx_unidade() AND comum.ctx_tem_perfil('ti','gerente'));
GRANT SELECT, INSERT, UPDATE ON identidade.pedido_perfil TO vidacor_app;

-- O cadastro de usuários continua legível pela sessão (login), mas só TI e gerência da MESMA unidade
-- escrevem nele: barreira no banco por gatilho (sem RLS de leitura aberta, que seria USING (true)).
CREATE FUNCTION identidade.usuario_escrita_permitida() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  -- TI não recebe perfil clínico (docs/18 § 6: TI sem dado clínico), venha de onde vier.
  IF 'ti' = ANY(NEW.perfis) AND NEW.perfis && ARRAY['medico','tecnico','enfermagem','auditor'] THEN
    RAISE EXCEPTION 'perfil de TI não acumula perfil clínico' USING ERRCODE = '23514';
  END IF;
  -- O papel da aplicação só escreve com contexto de TI ou gerência da mesma unidade. O dono do banco
  -- (migration, semente de dev, carga da migração revisada) não passa por aqui.
  IF current_user = 'vidacor_app' AND NOT (comum.ctx_tem_perfil('ti','gerente') AND NEW.unidade_id = comum.ctx_unidade()) THEN
    RAISE EXCEPTION 'escrita em identidade.usuario negada' USING ERRCODE = '42501';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER usuario_escrita BEFORE INSERT OR UPDATE ON identidade.usuario
  FOR EACH ROW EXECUTE FUNCTION identidade.usuario_escrita_permitida();
GRANT INSERT, UPDATE (perfis, ativo, nome, versao) ON identidade.usuario TO vidacor_app;
