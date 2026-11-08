#!/usr/bin/env python3
"""Confere a coerência dos pacientes fictícios nos mockups.

Falha (código 1) quando:
  - o mesmo paciente (primeiro e último nome) aparece com nascimentos diferentes;
  - a idade escrita ao lado do nascimento não bate com a data de referência dos mockups (07/11/2026).
Exceções documentadas em design/dados-ficticios.md (ex.: duplicado proposital da Maria Helena Duarte).
Uso: python3 tools/verificacao/dados_ficticios.py [pasta_dos_mockups]
"""
import collections, glob, os, re, sys

PASTA = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), '..', '..', 'design', 'mockups')
REF = (2026, 11, 7)
EXCECOES = {('Maria', 'Duarte')}  # duplicado proposital (21/03/1968) usado para demonstrar a busca de duplicados
NOME = r"([A-ZÁÉÍÓÚÂÊÔÃÕÇ][a-záéíóúâêôãõç]+(?: (?:d[aeo]s? )?[A-ZÁÉÍÓÚÂÊÔÃÕÇ][a-záéíóúâêôãõç]+\.?){1,3})"

datas = collections.defaultdict(lambda: collections.defaultdict(set))
idades = []
for f in sorted(glob.glob(os.path.join(PASTA, '*.dc.html'))):
    s = open(f, encoding='utf-8').read()
    base = os.path.basename(f)
    for m in re.finditer(NOME + r"[^0-9<]{0,40}?(\d{2}/\d{2}/(?:19\d{2}|200\d|201\d))", s):
        p = m.group(1).replace('.', '').split()
        datas[(p[0], p[-1])][m.group(2)].add(base)
    for m in re.finditer(r'(\d{2})/(\d{2})/(19\d{2}|20[01]\d)\W{1,4}(\d{1,3}) anos', s):
        d, mo, a, idade = map(int, m.groups())
        real = REF[0] - a - (1 if (mo, d) > (REF[1], REF[2]) else 0)
        if real != idade:
            idades.append(f"{base}: '{m.group(0)}' deveria ser {real} anos")

erros = []
for k, ds in sorted(datas.items()):
    if len(ds) > 1 and k not in EXCECOES:
        erros.append(f"{' '.join(k)}: " + '; '.join(f"{d} em {', '.join(sorted(fs))}" for d, fs in ds.items()))
for e in erros + idades:
    print(e)
print(f"{len(erros)} paciente(s) com nascimentos diferentes, {len(idades)} idade(s) errada(s)")
sys.exit(1 if erros or idades else 0)
