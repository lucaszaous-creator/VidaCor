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
-- Sexo do cadastro (TFG na triagem)
UPDATE identidade.paciente SET sexo = CASE WHEN nome IN ('Antônio Carlos Ribeiro','Rafael Monteiro Lima') THEN 'M' ELSE 'F' END WHERE sexo IS NULL;

-- Aparelhos por sala (worklist no AE Title do aparelho)
INSERT INTO agenda.aparelho (id, codigo, nome, ae_title, modalidade, sala_id, origem, unidade_id, correlacao)
SELECT gen_random_uuid(), 'APA-' || s.codigo, a.nome, a.ae, a.modalidade, s.id, 'semente', s.unidade_id, 'semente-dev'
FROM (VALUES ('SALA-1','Vivid E95','VIVIDE95','US'), ('SALA-2','Voluson E10','VOLUSON10','US'), ('SALA-3','LOGIQ E10','LOGIQE10','US'),
             ('SALA-5','Optima XR240','OPTIMAXR','DX'), ('SALA-6','Revolution CT','REVOCT','CT')) AS a(sala, nome, ae, modalidade)
JOIN agenda.sala s ON s.codigo = a.sala
ON CONFLICT DO NOTHING;

-- Lotação do técnico
INSERT INTO identidade.lotacao (usuario_id, sala_id, origem, unidade_id, correlacao)
SELECT u.id, s.id, 'semente', u.unidade_id, 'semente-dev' FROM identidade.usuario u JOIN agenda.sala s ON s.codigo IN ('SALA-1','SALA-2','SALA-3','SALA-6')
WHERE u.login = 'bruno.sales' ON CONFLICT DO NOTHING;

-- Médico dos agendamentos (vínculo de cuidado e laudo atribuído)
UPDATE agenda.agendamento SET medico_id = (SELECT id FROM identidade.usuario WHERE login = 'paulo.mendes') WHERE medico_id IS NULL;
UPDATE agenda.agendamento SET requer_contraste = true WHERE exame_nome ILIKE '%contraste%';

-- Tabela de preços do particular, versão 12
INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, origem, unidade_id, correlacao)
VALUES ('0194f000-0000-7000-8000-00000000c012', 12, DATE '2026-01-01', 'Particular 2026', 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev')
ON CONFLICT DO NOTHING;
INSERT INTO financeiro.preco (tabela_id, tuss, valor_centavos)
SELECT '0194f000-0000-7000-8000-00000000c012', tuss, valor FROM (VALUES ('40901114', 38000), ('40901300', 42000), ('40901203', 32000),
  ('40901122', 18000), ('40801012', 9000), ('41001109', 95000)) AS p(tuss, valor)
ON CONFLICT DO NOTHING;

-- Caixa e gerência para a demonstração
INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao)
SELECT gen_random_uuid(), 'USR-' || login, login, nome, perfis, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev'
FROM (VALUES ('carla.caixa', 'Carla Mendes', ARRAY['caixa']), ('patricia.lemos', 'Patrícia Lemos', ARRAY['gerente'])) AS u(login, nome, perfis)
ON CONFLICT (login) DO NOTHING;
-- O contador do cadastro começa depois dos códigos da semente (migration 0020).
INSERT INTO comum.contador (prefixo, ano, valor)
SELECT 'PAC', substring(codigo FROM 5 FOR 4)::int, max(substring(codigo FROM 10)::bigint)
  FROM identidade.paciente WHERE codigo ~ '^PAC-[0-9]{4}-[0-9]{6}$' GROUP BY 2
ON CONFLICT (prefixo, ano) DO UPDATE SET valor = GREATEST(comum.contador.valor, EXCLUDED.valor);
-- E-mails fictícios (domínio .invalid não existe) para o convite do portal ir ao Mailpit.
UPDATE identidade.paciente SET email = lower(translate(split_part(nome, ' ', 1) || '.' || reverse(split_part(reverse(nome), ' ', 1)),
  'áàâãéêíóôõúçÁÉÍÓÚÂÊÔÃÕÇ', 'aaaaeeiooouucAEIOUAEOAOC')) || '@exemplo.invalid'
 WHERE email IS NULL AND origem = 'semente';
-- Estoque fictício (Estoque.dc.html): contraste e consumíveis, com lotes e validades.
INSERT INTO estoque.item (id, codigo, nome, medida, minimo, local, contraste, origem, unidade_id, correlacao)
VALUES ('0194f000-0000-7000-8000-0000000e0001', 'ITM-CTR', 'Contraste iodado não iônico 350, 100 ml', 'frasco', 20, 'Tomografia', true, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev'),
       ('0194f000-0000-7000-8000-0000000e0002', 'ITM-GEL', 'Gel para ultrassom, 5 L', 'galao', 6, 'Salas de USG', false, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev'),
       ('0194f000-0000-7000-8000-0000000e0003', 'ITM-SER', 'Seringa 20 ml com scalp 21G', 'kit', 80, 'Tomografia', false, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev')
ON CONFLICT (codigo) DO NOTHING;
INSERT INTO estoque.lote (id, item_id, numero, validade, saldo, origem, unidade_id, correlacao)
VALUES (gen_random_uuid(), '0194f000-0000-7000-8000-0000000e0001', 'L2408-77', DATE '2027-03-31', 8, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev'),
       (gen_random_uuid(), '0194f000-0000-7000-8000-0000000e0002', 'G0911', DATE '2026-12-15', 4, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev'),
       (gen_random_uuid(), '0194f000-0000-7000-8000-0000000e0002', 'G1003', DATE '2027-10-31', 10, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev'),
       (gen_random_uuid(), '0194f000-0000-7000-8000-0000000e0003', 'S8820', DATE '2029-02-28', 240, 'semente', '0194f000-0000-7000-8000-0000000000a1', 'semente-dev')
ON CONFLICT (item_id, numero) DO NOTHING;
COMMIT;
