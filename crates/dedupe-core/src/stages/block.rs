//! `__splink__blocked_id_pairs`: one UNION ALL arm per blocking rule, with
//! `match_key` = rule index as a string literal and the cross-rule dedup
//! trick. Matches golden `predict_01___splink__blocked_id_pairs.sql`.
//!
//! v5 blocks on `__splink__df_concat` (TF is not needed for blocking).

use super::UidExpr;
use crate::blocking::BlockingRule;
use crate::settings::json::LinkType;

pub const BLOCKED_ID_PAIRS: &str = "__splink__blocked_id_pairs";

/// The WHERE condition by link type (`_sql_gen_where_condition`):
/// - dedupe_only / link_and_dedupe: `where {uid_l} < {uid_r}`
/// - link_only: additionally `and l.{sd} != r.{sd}`
/// - two-dataset link_only: `where 1=1` (handled when link support lands)
fn where_condition(uid: &UidExpr, link_type: LinkType, source_dataset_col: &str) -> String {
    let base = format!("where {} < {}", uid.render("l"), uid.render("r"));
    match link_type {
        LinkType::DedupeOnly | LinkType::LinkAndDedupe => base,
        LinkType::LinkOnly => {
            format!("{base} and l.\"{source_dataset_col}\" != r.\"{source_dataset_col}\"")
        }
    }
}

fn exclude_preceding_sql(rule: &BlockingRule, preceding: &[&BlockingRule]) -> String {
    let _ = rule;
    if preceding.is_empty() {
        return String::new();
    }
    let clauses: Vec<String> = preceding
        .iter()
        .map(|r| format!("coalesce(({}),false)", r.sql))
        .collect();
    format!("\nAND NOT ({})", clauses.join(" OR "))
}

pub fn blocked_id_pairs_sql(
    rules: &[BlockingRule],
    uid: &UidExpr,
    link_type: LinkType,
    source_dataset_col: &str,
    input_table: &str,
) -> String {
    // No rules -> cartesian product via a literal 1=1 rule (Splink's fallback).
    let fallback;
    let rules: &[BlockingRule] = if rules.is_empty() {
        fallback = [BlockingRule {
            index: 0,
            sql: "1=1".to_string(),
        }];
        &fallback
    } else {
        rules
    };

    let arms: Vec<String> = rules
        .iter()
        .enumerate()
        .map(|(i, rule)| {
            let preceding: Vec<&BlockingRule> = rules[..i].iter().collect();
            format!(
                "select\n'{match_key}' as match_key,\n{uid_l} as join_key_l,\n{uid_r} as join_key_r\nfrom {input_table} as l\ninner join {input_table} as r\non\n({rule_sql})\n{where}{exclude}",
                match_key = rule.index,
                uid_l = uid.render("l"),
                uid_r = uid.render("r"),
                rule_sql = rule.sql,
                where = where_condition(uid, link_type, source_dataset_col),
                exclude = exclude_preceding_sql(rule, &preceding),
            )
        })
        .collect();
    arms.join("\n\nUNION ALL\n\n")
}
