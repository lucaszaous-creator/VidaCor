//! Consultas dos relatórios (catálogo fechado). Cada uma roda na transação com o contexto do usuário:
//! o RLS continua decidindo o que ele lê. Nenhuma devolve nome de paciente nem nome de exame em
//! relatório com dado de saúde: o paciente vira o código PAC e o exame, o código TUSS.

use crate::erro::ErroApi;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgConnection;
use utoipa::ToSchema;

/// Definição de um relatório do catálogo.
pub struct Definicao {
    pub id: &'static str,
    pub grupo: &'static str,
    pub titulo: &'static str,
    pub descricao: &'static str,
    pub colunas: [&'static str; 3],
    /// Tinha dado de saúde: sai pseudonimizado (PAC e TUSS).
    pub saude: bool,
    /// Perfis que podem ver e exportar.
    pub perfis: &'static [&'static str],
}

pub const CATALOGO: [Definicao; 7] = [
    Definicao {
        id: "producao_medico",
        grupo: "Produção",
        titulo: "Produção por médico",
        descricao: "Laudos assinados por médico no período, com o exame mais frequente",
        colunas: ["Médico", "Principal exame", "Laudos"],
        saude: false,
        perfis: &["gerente"],
    },
    Definicao {
        id: "producao_convenio",
        grupo: "Produção",
        titulo: "Produção por convênio",
        descricao: "Guias e valor apresentado por operadora (TISS), pela data de execução",
        colunas: ["Convênio", "Valor apresentado", "Guias"],
        saude: false,
        perfis: &["gerente", "faturista"],
    },
    Definicao {
        id: "producao_sala",
        grupo: "Produção",
        titulo: "Produção por sala",
        descricao: "Exames realizados e ocupação de cada sala (grade de 07:00 às 19:00)",
        colunas: ["Sala", "Ocupação", "Exames"],
        saude: false,
        perfis: &["gerente"],
    },
    Definicao {
        id: "faltas",
        grupo: "Agenda",
        titulo: "Faltas",
        descricao: "Pacientes que faltaram, por código do paciente e do exame",
        colunas: ["Paciente", "Exame", "Faltas"],
        saude: true,
        perfis: &["gerente"],
    },
    Definicao {
        id: "glosas",
        grupo: "Faturamento",
        titulo: "Glosas",
        descricao: "Valor glosado por convênio e o motivo mais frequente",
        colunas: ["Convênio", "Motivo principal", "Glosado (R$)"],
        saude: false,
        perfis: &["gerente", "faturista"],
    },
    Definicao {
        id: "financeiro",
        grupo: "Faturamento",
        titulo: "Recebido no caixa",
        descricao: "Recebido no caixa do particular por forma de pagamento",
        colunas: ["Forma", "Participação", "Recebido (R$)"],
        saude: false,
        perfis: &["gerente", "financeiro"],
    },
    Definicao {
        id: "estoque",
        grupo: "Operação",
        titulo: "Estoque",
        descricao: "Consumo no período e a validade mais próxima com saldo",
        colunas: ["Item", "Validade mais próxima", "Consumo"],
        saude: false,
        perfis: &["gerente", "estoque"],
    },
];

#[derive(Serialize, ToSchema, Clone)]
pub struct LinhaDoRelatorio {
    pub a: String,
    pub b: String,
    /// Valor numérico da terceira coluna (centavos nos relatórios em reais).
    pub valor: i64,
    pub c: String,
}

#[must_use]
pub fn buscar(id: &str) -> Option<&'static Definicao> {
    CATALOGO.iter().find(|d| d.id == id)
}

/// Reais a partir de centavos, no formato brasileiro (puro).
#[must_use]
pub fn reais(centavos: i64) -> String {
    let inteiro = centavos / 100;
    let resto = (centavos % 100).abs();
    let digitos = inteiro.abs().to_string();
    let mut com_ponto = String::new();
    for (i, c) in digitos.chars().enumerate() {
        if i > 0 && (digitos.len() - i) % 3 == 0 {
            com_ponto.push('.');
        }
        com_ponto.push(c);
    }
    format!(
        "{}{com_ponto},{resto:02}",
        if centavos < 0 { "-" } else { "" }
    )
}

fn linha(a: String, b: String, valor: i64, c: String) -> LinhaDoRelatorio {
    LinhaDoRelatorio { a, b, valor, c }
}

/// Linhas do relatório no período (já conferida a permissão do perfil).
///
/// # Errors
/// Falha de banco.
pub async fn linhas(
    tx: &mut PgConnection,
    id: &str,
    de: DateTime<Utc>,
    ate: DateTime<Utc>,
    dias: i64,
) -> Result<Vec<LinhaDoRelatorio>, ErroApi> {
    let r = match id {
        "producao_medico" => sqlx::query!(
            r#"SELECT medico AS "medico!", principal_exame, laudos AS "laudos!" FROM relatorios.producao_por_medico($1, $2)"#,
            de, ate
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|l| linha(l.medico, l.principal_exame.unwrap_or_default(), l.laudos, l.laudos.to_string()))
        .collect(),
        "producao_convenio" => sqlx::query!(
            r#"SELECT operadora, count(*) AS "guias!", coalesce(sum(valor_total_centavos), 0)::bigint AS "valor!"
                 FROM faturamento.conta
                WHERE data_execucao >= ($1 AT TIME ZONE 'America/Sao_Paulo')::date
                  AND data_execucao < ($2 AT TIME ZONE 'America/Sao_Paulo')::date
                GROUP BY operadora ORDER BY 3 DESC LIMIT 200"#,
            de, ate
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|l| linha(l.operadora, format!("R$ {}", reais(l.valor)), l.guias, l.guias.to_string()))
        .collect(),
        "producao_sala" => {
            let abertos = 12 * 60 * dias.max(1);
            sqlx::query!(
                r#"SELECT s.nome,
                          count(a.id) FILTER (WHERE a.situacao = 'concluido' OR t.situacao = 'exame_concluido') AS "exames!",
                          coalesce(sum(extract(epoch FROM (a.fim - a.inicio)) / 60)
                                   FILTER (WHERE a.situacao NOT IN ('cancelado','expirada')), 0)::bigint AS "minutos!"
                     FROM agenda.sala s
                     LEFT JOIN agenda.agendamento a ON a.sala_id = s.id AND a.inicio >= $1 AND a.inicio < $2
                     LEFT JOIN recepcao.atendimento t ON t.agendamento_id = a.id
                    WHERE s.unidade_id = comum.ctx_unidade()
                    GROUP BY s.id, s.nome ORDER BY s.nome LIMIT 100"#,
                de, ate
            )
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|l| linha(l.nome, format!("{}%", (l.minutos * 100 / abertos).min(100)), l.exames, l.exames.to_string()))
            .collect()
        }
        "faltas" => sqlx::query!(
            r#"SELECT p.codigo AS paciente, a.exame_tuss, count(*) AS "faltas!"
                 FROM agenda.agendamento a JOIN identidade.paciente p ON p.id = a.paciente_id
                WHERE a.situacao = 'faltou' AND a.inicio >= $1 AND a.inicio < $2
                GROUP BY p.codigo, a.exame_tuss ORDER BY 3 DESC, 1 LIMIT 500"#,
            de, ate
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|l| linha(l.paciente, format!("TUSS {}", l.exame_tuss), l.faltas, l.faltas.to_string()))
        .collect(),
        "glosas" => sqlx::query!(
            r#"SELECT c.operadora, coalesce(sum(g.valor_centavos), 0)::bigint AS "valor!",
                      (SELECT g2.motivo FROM faturamento.glosa g2 JOIN faturamento.conta c2 ON c2.id = g2.conta_id
                        WHERE c2.operadora = c.operadora AND g2.criado_em >= $1 AND g2.criado_em < $2
                        GROUP BY g2.motivo ORDER BY count(*) DESC, g2.motivo LIMIT 1) AS motivo
                 FROM faturamento.glosa g JOIN faturamento.conta c ON c.id = g.conta_id
                WHERE g.criado_em >= $1 AND g.criado_em < $2
                GROUP BY c.operadora ORDER BY 2 DESC LIMIT 200"#,
            de, ate
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|l| linha(l.operadora, l.motivo.unwrap_or_default(), l.valor, reais(l.valor)))
        .collect(),
        "financeiro" => {
            let formas = sqlx::query!(
                r#"SELECT forma, coalesce(sum(total_centavos), 0)::bigint AS "valor!"
                     FROM caixa.recebimento WHERE situacao = 'pago' AND criado_em >= $1 AND criado_em < $2
                    GROUP BY forma ORDER BY 2 DESC"#,
                de, ate
            )
            .fetch_all(&mut *tx)
            .await?;
            let total: i64 = formas.iter().map(|f| f.valor).sum();
            formas
                .into_iter()
                .map(|f| {
                    let nome = match f.forma.as_str() {
                        "pix" => "PIX",
                        "cartao" => "Cartão",
                        _ => "Dinheiro",
                    };
                    let parte = if total > 0 { f.valor * 100 / total } else { 0 };
                    linha(nome.to_owned(), format!("{parte}%"), f.valor, reais(f.valor))
                })
                .collect()
        }
        "estoque" => sqlx::query!(
            r#"SELECT i.nome,
                      (SELECT min(l2.validade) FROM estoque.lote l2 WHERE l2.item_id = i.id AND l2.saldo > 0) AS validade,
                      coalesce((SELECT sum(m.quantidade) FROM estoque.movimento m JOIN estoque.lote l ON l.id = m.lote_id
                                 WHERE l.item_id = i.id AND m.tipo IN ('baixa_exame','baixa_manual')
                                   AND m.criado_em >= $1 AND m.criado_em < $2), 0)::bigint AS "consumo!"
                 FROM estoque.item i WHERE i.ativo ORDER BY 3 DESC, i.nome LIMIT 300"#,
            de, ate
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|l| {
            let v = l.validade.map_or_else(|| "Sem saldo".to_owned(), |d| d.format("%m/%Y").to_string());
            linha(l.nome, v, l.consumo, l.consumo.to_string())
        })
        .collect(),
        _ => Vec::new(),
    };
    Ok(r)
}

/// CSV para planilha em português: separador `;`, BOM UTF-8, aspas quando preciso e células que
/// começam com `=`, `+`, `-` ou `@` neutralizadas (injeção de fórmula, SEC-INJ).
#[must_use]
pub fn csv(colunas: &[&str; 3], linhas: &[LinhaDoRelatorio]) -> Vec<u8> {
    fn celula(s: &str) -> String {
        let neutra = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
            format!("'{s}")
        } else {
            s.to_owned()
        };
        if neutra.contains([';', '"', '\n', '\r']) {
            format!("\"{}\"", neutra.replace('"', "\"\""))
        } else {
            neutra
        }
    }
    let mut out = String::from("\u{feff}");
    out.push_str(
        &colunas
            .iter()
            .map(|c| celula(c))
            .collect::<Vec<_>>()
            .join(";"),
    );
    out.push_str("\r\n");
    for l in linhas {
        out.push_str(&[celula(&l.a), celula(&l.b), celula(&l.c)].join(";"));
        out.push_str("\r\n");
    }
    out.into_bytes()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn reais_no_formato_brasileiro() {
        assert_eq!(reais(0), "0,00");
        assert_eq!(reais(42_100), "421,00");
        assert_eq!(reais(14_832_045), "148.320,45");
        assert_eq!(reais(-1_234_567), "-12.345,67");
    }

    #[test]
    fn csv_neutraliza_formula_e_escapa() {
        let l = vec![linha("=CMD()".into(), "a;b".into(), 1, "1".into())];
        let s = String::from_utf8(csv(&["A", "B", "C"], &l)).unwrap_or_default();
        assert!(s.starts_with('\u{feff}'));
        assert!(s.contains("'=CMD();\"a;b\";1\r\n"));
    }

    #[test]
    fn catalogo_com_perfis_e_saude() {
        assert!(buscar("faltas").is_some_and(|d| d.saude));
        assert!(buscar("glosas").is_some_and(|d| d.perfis.contains(&"faturista")));
        assert!(buscar("inexistente").is_none());
        for d in &CATALOGO {
            assert!(!d.perfis.contains(&"ti"), "{}", d.id);
            assert!(!d.perfis.contains(&"caixa"), "{}", d.id);
        }
    }
}
