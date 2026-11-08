-- Dados FICTÍCIOS de desenvolvimento (design/dados-ficticios.md). Nunca rodar em produção.
-- Agenda montada para o dia corrente em America/Sao_Paulo. Idempotente: pode rodar de novo.
BEGIN;
INSERT INTO identidade.unidade (id, codigo, nome, origem, unidade_id, correlacao) VALUES
  ('0194f000-0000-7000-8000-0000000000a1', 'UNI-CENTRO', 'Centro', 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev')
ON CONFLICT DO NOTHING;

INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao)
SELECT gen_random_uuid(), 'USR-' || login, login, nome, perfis, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev'
FROM (VALUES
  ('ana.souza', 'Ana Souza', ARRAY['recepcao']),
  ('carla.mendes', 'Carla Mendes', ARRAY['recepcao']),
  ('bruno.sales', 'Bruno Sales', ARRAY['tecnico']),
  ('paulo.mendes', 'Dr. Paulo Mendes', ARRAY['medico']),
  ('ti.vidacor', 'Equipe TI', ARRAY['ti'])
) AS u(login, nome, perfis)
ON CONFLICT (login) DO NOTHING;

INSERT INTO agenda.sala (id, codigo, nome, ae_title, modalidade, origem, unidade_id, correlacao)
SELECT gen_random_uuid(), codigo, nome, ae, modalidade, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev'
FROM (VALUES
  ('SALA-1', 'Sala 1', 'SALA1', 'US'), ('SALA-2', 'Sala 2', 'SALA2', 'US'),
  ('SALA-3', 'Sala 3', 'SALA3', 'US'), ('SALA-4', 'Sala 4', 'SALA4', 'ECG'),
  ('SALA-5', 'Sala 5', 'SALA5', 'DX'), ('SALA-6', 'Sala 6', 'SALA6', 'CT'),
  ('POSTO-MAPA', 'Posto MAPA e Holter', 'MAPA', 'OT')
) AS s(codigo, nome, ae, modalidade)
ON CONFLICT (codigo) DO NOTHING;

INSERT INTO identidade.paciente (id, codigo, nome, nascimento, origem, unidade_id, correlacao)
SELECT gen_random_uuid(), codigo, nome, nasc::date, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev'
FROM (VALUES
  ('PAC-2026-000001', 'Maria Helena Duarte', '1968-03-12'),
  ('PAC-2026-000002', 'Antônio Carlos Ribeiro', '1954-06-15'),
  ('PAC-2026-000003', 'Juliana Sampaio Rocha', '1993-04-12'),
  ('PAC-2026-000004', 'Lúcia Fernandes Prado', '1959-07-02'),
  ('PAC-2026-000005', 'Sônia Maria Castro', '1962-02-03'),
  ('PAC-2026-000006', 'Rafael Monteiro Lima', '1988-11-09'),
  ('PAC-2026-000007', 'Rita Costa Barbosa', '1966-09-09'),
  ('PAC-2026-000008', 'Maria Helena Duarte', '1968-03-21')
) AS p(codigo, nome, nasc)
ON CONFLICT (codigo) DO NOTHING;

-- Agenda de hoje
INSERT INTO agenda.agendamento (id, codigo, sala_id, paciente_id, inicio, fim, exame_tuss, exame_nome, situacao, origem, unidade_id, correlacao)
SELECT gen_random_uuid(),
       'AGD-DEV-' || to_char((now() AT TIME ZONE 'America/Sao_Paulo')::date, 'YYYYMMDD') || '-' || a.n,
       s.id, p.id,
       ((now() AT TIME ZONE 'America/Sao_Paulo')::date + a.hora) AT TIME ZONE 'America/Sao_Paulo',
       ((now() AT TIME ZONE 'America/Sao_Paulo')::date + a.hora + interval '30 minutes') AT TIME ZONE 'America/Sao_Paulo',
       a.tuss, a.exame, a.situacao, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev'
FROM (VALUES
  ('01', time '07:30', 'SALA-1', 'PAC-2026-000001', '40901114', 'Ecocardiograma transtorácico', 'confirmado'),
  ('02', time '08:00', 'SALA-1', 'PAC-2026-000002', '40901300', 'Ecodoppler de carótidas', 'confirmado'),
  ('03', time '08:30', 'SALA-2', 'PAC-2026-000003', '40901203', 'USG obstétrica morfológica', 'agendado'),
  ('04', time '09:00', 'SALA-6', 'PAC-2026-000004', '41001109', 'TC de tórax com contraste', 'agendado'),
  ('05', time '09:30', 'SALA-3', 'PAC-2026-000007', '40901122', 'USG de tireoide', 'confirmado'),
  ('06', time '10:00', 'SALA-5', 'PAC-2026-000006', '40801012', 'Raio X de tórax', 'agendado'),
  ('07', time '10:30', 'SALA-1', 'PAC-2026-000005', '40901114', 'Ecocardiograma transtorácico', 'agendado')
) AS a(n, hora, sala, paciente, tuss, exame, situacao)
JOIN agenda.sala s ON s.codigo = a.sala
JOIN identidade.paciente p ON p.codigo = a.paciente
ON CONFLICT (codigo) DO NOTHING;
COMMIT;
