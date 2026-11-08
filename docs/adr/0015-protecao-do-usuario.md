# ADR-0015 · Proteção do usuário: a cerca e a rede de segurança

- **Status:** aceito (aprovado pelo dono do produto em 2026-11-08)
- **Data:** 2026-11-07

## Contexto
Em um sistema clínico, o erro que chega ao usuário pode virar dano ao paciente (identificação errada, resultado no lugar errado) ou parar o atendimento. Regras de código (G1–G6) não bastam: são precisos procedimentos de construção, teste, lançamento e operação, e requisitos de interface que impeçam o erro humano.

## Decisão
Adotar o [docs/16](../16-protecao-do-usuario.md) como regra obrigatória: papéis (incluindo responsável clínico de segurança), análise de risco clínico, rastreabilidade, perfis IHE, dois identificadores, testes de usabilidade com a equipe, caos em homologação, revisão de prontidão, piloto com análise canário, liberação gradual, pós-incidente sem culpados, e os requisitos de interface R1–R14. Modelos em [docs/modelos/](../modelos/).

## Consequências
Mais etapas antes de cada lançamento e envolvimento de um médico da clínica nas mudanças clínicas; em troca, erros são barrados antes do usuário e, quando algo escapa, o usuário não é prejudicado nem levado ao erro.
