//! DECIMAL 兼容层。
//!
//! taos-ws 驱动收到 DECIMAL 类型字段时解析即 panic（且连接随之挂死）。
//! 本模块在执行前对"简单 SELECT"（单表、无 JOIN/子查询）自动把
//! DECIMAL 列改写为 `CAST(col AS VARCHAR) AS col`，用户无需手动 CAST；
//! 改写无法覆盖的复杂语句由 REST API 兜底（见 commands::rest_query_one）。

// ---------- tokenizer ----------

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokKind {
    Ident,
    QuotedIdent,
    Str,
    Num,
    Punct,
}

#[derive(Debug, Clone)]
struct Tok {
    kind: TokKind,
    text: String,
    /// char 下标（用于回切原文）
    start: usize,
    end: usize,
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$' || !c.is_ascii()
}

fn tokenize(sql: &str) -> Vec<Tok> {
    let chars: Vec<char> = sql.chars().collect();
    let len = chars.len();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < len {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // 行注释 -- ...
        if c == '-' && i + 1 < len && chars[i + 1] == '-' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        // 块注释 /* ... */
        if c == '/' && i + 1 < len && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(len);
            continue;
        }
        // 反引号标识符（`` 转义）
        if c == '`' {
            let start = i;
            i += 1;
            let mut name = String::new();
            while i < len {
                if chars[i] == '`' {
                    if i + 1 < len && chars[i + 1] == '`' {
                        name.push('`');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                name.push(chars[i]);
                i += 1;
            }
            toks.push(Tok {
                kind: TokKind::QuotedIdent,
                text: name,
                start,
                end: i,
            });
            continue;
        }
        // 字符串字面量（'' 转义）
        if c == '\'' {
            let start = i;
            i += 1;
            let mut s = String::new();
            while i < len {
                if chars[i] == '\'' {
                    if i + 1 < len && chars[i + 1] == '\'' {
                        s.push('\'');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                s.push(chars[i]);
                i += 1;
            }
            toks.push(Tok {
                kind: TokKind::Str,
                text: s,
                start,
                end: i,
            });
            continue;
        }
        // 标识符 / 数字
        if is_ident_start(c) || c.is_ascii_digit() {
            let start = i;
            let mut s = String::new();
            while i < len && (is_ident_start(chars[i]) || chars[i].is_ascii_digit()) {
                s.push(chars[i]);
                i += 1;
            }
            let kind = if c.is_ascii_digit() {
                TokKind::Num
            } else {
                TokKind::Ident
            };
            toks.push(Tok {
                kind,
                text: s,
                start,
                end: i,
            });
            continue;
        }
        // 单字符标点
        toks.push(Tok {
            kind: TokKind::Punct,
            text: c.to_string(),
            start: i,
            end: i + 1,
        });
        i += 1;
    }
    toks
}

fn kw_eq(tok: &Tok, kw: &str) -> bool {
    tok.kind == TokKind::Ident && tok.text.eq_ignore_ascii_case(kw)
}

/// FROM 之后不允许作为表别名的子句/连接关键字
fn is_clause_kw(s: &str) -> bool {
    const KWS: &[&str] = &[
        "where", "group", "order", "limit", "having", "union", "join", "on", "slimit",
        "partition", "interval", "session", "state_window", "fill", "sliding", "by",
        "left", "right", "inner", "full", "cross", "natural", "as", "window",
    ];
    KWS.iter().any(|k| s.eq_ignore_ascii_case(k))
}

// ---------- simple SELECT 解析 ----------

pub struct SimpleSelect {
    pub table_db: Option<String>,
    pub table: String,
    pub alias: Option<String>,
    /// select list 的 token（不含 SELECT / FROM 关键字）
    list_toks: Vec<Tok>,
    /// 表引用结束后的剩余原文起始 char 下标（WHERE / ORDER / LIMIT ...）
    tail_start: usize,
}

/// 识别"简单 SELECT"：单表、无 JOIN / UNION / 子查询 / 多表逗号。
/// 无法识别返回 None（调用方保持原样执行）。
pub fn parse_simple_select(sql: &str) -> Option<SimpleSelect> {
    let toks = tokenize(sql);

    // 必须以 SELECT 开头
    let first = toks.first()?;
    if !kw_eq(first, "select") {
        return None;
    }

    // 顶层（括号深度 0）扫描：定位唯一的 FROM，遇到 UNION/JOIN 等直接放弃
    let mut depth = 0usize;
    let mut from_idx: Option<usize> = None;
    for (i, t) in toks.iter().enumerate().skip(1) {
        match t.kind {
            TokKind::Punct if t.text == "(" => depth += 1,
            TokKind::Punct if t.text == ")" => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 {
            if kw_eq(t, "from") {
                if from_idx.is_some() {
                    return None; // 两个顶层 FROM（UNION / 子查询）→ 复杂
                }
                from_idx = Some(i);
            }
            if ["union", "join", "intersect", "minus"]
                .iter()
                .any(|k| kw_eq(t, k))
            {
                return None;
            }
        }
    }
    let from_idx = from_idx?;

    // select list 中出现子查询（任意深度的 SELECT）→ 无法安全改写
    for t in &toks[1..from_idx] {
        if kw_eq(t, "select") {
            return None;
        }
    }

    // 解析表引用：[db.]table
    let mut i = from_idx + 1;
    let part1 = match toks.get(i) {
        Some(t) if matches!(t.kind, TokKind::Ident | TokKind::QuotedIdent) => {
            i += 1;
            t
        }
        _ => return None,
    };
    let (table_db, table) = if let Some(t) = toks.get(i) {
        if t.kind == TokKind::Punct && t.text == "." {
            i += 1;
            match toks.get(i) {
                Some(t2) if matches!(t2.kind, TokKind::Ident | TokKind::QuotedIdent) => {
                    i += 1;
                    (Some(part1.text.clone()), t2.text.clone())
                }
                _ => return None,
            }
        } else {
            (None, part1.text.clone())
        }
    } else {
        (None, part1.text.clone())
    };

    // 可选别名
    let mut alias: Option<String> = None;
    if let Some(t) = toks.get(i) {
        if matches!(t.kind, TokKind::Ident | TokKind::QuotedIdent) {
            if t.kind == TokKind::Ident && t.text.eq_ignore_ascii_case("as") {
                match toks.get(i + 1) {
                    Some(t2) if matches!(t2.kind, TokKind::Ident | TokKind::QuotedIdent) => {
                        alias = Some(t2.text.clone());
                        i += 2;
                    }
                    _ => return None,
                }
            } else if t.kind == TokKind::QuotedIdent || !is_clause_kw(&t.text) {
                alias = Some(t.text.clone());
                i += 1;
            }
        }
    }

    // 表引用后紧跟逗号 → 多表，放弃
    if let Some(t) = toks.get(i) {
        if t.kind == TokKind::Punct && t.text == "," {
            return None;
        }
    }

    // WHERE / 子句中出现子查询 → 其结果类型未知，放弃改写（交给 REST 兜底）
    for t in &toks[i..] {
        if kw_eq(t, "select") {
            return None;
        }
    }

    let list_toks = toks[1..from_idx].to_vec();
    let tail_start = toks.get(i).map(|t| t.start).unwrap_or(sql.chars().count());

    Some(SimpleSelect {
        table_db,
        table,
        alias,
        list_toks,
        tail_start,
    })
}

// ---------- select list 改写 ----------

pub struct ColumnType {
    pub name: String,
    pub ty: String,
}

enum ItemKind {
    Star,
    QualifiedStar(String),
    PlainCol(String),
    QualifiedCol(String, String),
    Expr,
}

fn quote_name(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

fn cast_expr(name: &str) -> String {
    format!(
        "CAST({} AS VARCHAR) AS {}",
        quote_name(name),
        quote_name(name)
    )
}

/// 顶层逗号拆分 select list（跳过括号嵌套）
fn split_items(toks: &[Tok]) -> Option<Vec<Vec<Tok>>> {
    let mut items: Vec<Vec<Tok>> = Vec::new();
    let mut cur: Vec<Tok> = Vec::new();
    let mut depth = 0usize;
    for t in toks {
        if t.kind == TokKind::Punct {
            match t.text.as_str() {
                "(" => depth += 1,
                ")" => {
                    if depth == 0 {
                        return None; // 括号不配对
                    }
                    depth -= 1;
                }
                "," if depth == 0 => {
                    items.push(std::mem::take(&mut cur));
                    continue;
                }
                _ => {}
            }
        }
        cur.push(t.clone());
    }
    if depth != 0 {
        return None;
    }
    if !cur.is_empty() || !items.is_empty() {
        items.push(cur);
    }
    Some(items)
}

fn classify_item(item: &[Tok]) -> ItemKind {
    match item.len() {
        1 => match &item[0].kind {
            TokKind::Punct if item[0].text == "*" => ItemKind::Star,
            TokKind::Ident | TokKind::QuotedIdent => {
                ItemKind::PlainCol(item[0].text.clone())
            }
            _ => ItemKind::Expr,
        },
        3 => {
            if item[1].kind == TokKind::Punct && item[1].text == "." {
                let head_ok =
                    matches!(item[0].kind, TokKind::Ident | TokKind::QuotedIdent);
                if item[2].kind == TokKind::Punct && item[2].text == "*" && head_ok {
                    ItemKind::QualifiedStar(item[0].text.clone())
                } else if head_ok
                    && matches!(item[2].kind, TokKind::Ident | TokKind::QuotedIdent)
                {
                    ItemKind::QualifiedCol(item[0].text.clone(), item[2].text.clone())
                } else {
                    ItemKind::Expr
                }
            } else {
                ItemKind::Expr
            }
        }
        _ => ItemKind::Expr,
    }
}

/// 用原文片段重建 select 项（保留原始写法）
fn rebuild_item(sql: &str, item: &[Tok]) -> String {
    match (item.first(), item.last()) {
        (Some(a), Some(b)) => {
            let chars: Vec<char> = sql.chars().collect();
            chars[a.start..b.end].iter().collect()
        }
        _ => String::new(),
    }
}

/// 把简单 SELECT 中的 DECIMAL 列改写为 CAST(... AS VARCHAR)。
/// `use_db`：当前 USE 的库（SQL 中未带库前缀时用于补全 `db.table`，避免上下文丢失）。
/// 返回 None 表示无需改写或不能安全改写。
pub fn rewrite(sql: &str, p: &SimpleSelect, cols: &[ColumnType], use_db: Option<&str>) -> Option<String> {
    // DECIMAL 列集合（TDengine 标识符大小写不敏感）
    let decimal_cols: std::collections::HashSet<String> = cols
        .iter()
        .filter(|c| c.ty.to_ascii_lowercase().starts_with("decimal"))
        .map(|c| c.name.to_ascii_lowercase())
        .collect();
    if decimal_cols.is_empty() {
        return None; // 无 DECIMAL 列，无需改写
    }

    // 表名与别名均可作为限定前缀
    let mut prefixes = vec![p.table.to_ascii_lowercase()];
    if let Some(a) = &p.alias {
        prefixes.push(a.to_ascii_lowercase());
    }

    let items = split_items(&p.list_toks)?;
    let mut out_items: Vec<String> = Vec::with_capacity(items.len());

    let expand_all = |out: &mut Vec<String>| {
        let parts: Vec<String> = cols
            .iter()
            .map(|c| {
                if decimal_cols.contains(&c.name.to_ascii_lowercase()) {
                    cast_expr(&c.name)
                } else {
                    quote_name(&c.name)
                }
            })
            .collect();
        out.push(parts.join(", "));
    };

    for item in &items {
        match classify_item(item) {
            ItemKind::Star => expand_all(&mut out_items),
            ItemKind::QualifiedStar(prefix) => {
                if prefixes.contains(&prefix.to_ascii_lowercase()) {
                    expand_all(&mut out_items);
                } else {
                    return None; // 未知前缀，放弃
                }
            }
            ItemKind::PlainCol(name) => {
                if decimal_cols.contains(&name.to_ascii_lowercase()) {
                    out_items.push(cast_expr(&name));
                } else {
                    out_items.push(rebuild_item(sql, item));
                }
            }
            ItemKind::QualifiedCol(prefix, name) => {
                if prefixes.contains(&prefix.to_ascii_lowercase())
                    && decimal_cols.contains(&name.to_ascii_lowercase())
                {
                    out_items.push(cast_expr(&name));
                } else {
                    out_items.push(rebuild_item(sql, item));
                }
            }
            ItemKind::Expr => {
                // 表达式若引用了 DECIMAL 列（如 val+1、avg(val)），
                // 其结果可能仍是 DECIMAL，无法安全改写 → 整体放弃（交给 REST 兜底）
                for t in item {
                    if matches!(t.kind, TokKind::Ident | TokKind::QuotedIdent)
                        && decimal_cols.contains(&t.text.to_ascii_lowercase())
                    {
                        return None;
                    }
                }
                out_items.push(rebuild_item(sql, item));
            }
        }
    }

    if out_items.is_empty() {
        return None;
    }

    // 重组：SELECT ... FROM [`db`.]`table` [alias] <tail 原文>
    let chars: Vec<char> = sql.chars().collect();
    let tail: String = chars[p.tail_start.min(chars.len())..].iter().collect();
    let alias_part = p
        .alias
        .as_ref()
        .map(|a| format!(" {} ", quote_name(a)))
        .unwrap_or_else(|| " ".to_string());
    let table_ref = match p.table_db.as_deref().or(use_db) {
        Some(d) => format!("{}.{}", quote_name(d), quote_name(&p.table)),
        None => quote_name(&p.table),
    };
    Some(format!(
        "SELECT {} FROM {}{}{}",
        out_items.join(", "),
        table_ref,
        alias_part,
        tail
    ))
}

// ---------- 单元测试 ----------

#[cfg(test)]
mod tests {
    use super::*;

    fn cols() -> Vec<ColumnType> {
        vec![
            ColumnType { name: "ts".into(), ty: "TIMESTAMP".into() },
            ColumnType { name: "val".into(), ty: "DECIMAL(10,4)".into() },
            ColumnType { name: "memo".into(), ty: "VARCHAR(32)".into() },
        ]
    }

    fn rewrite_sql(sql: &str) -> Option<String> {
        let p = parse_simple_select(sql)?;
        rewrite(sql, &p, &cols(), None)
    }

    #[test]
    fn star_expand() {
        let out = rewrite_sql("select * from t1 limit 10").unwrap();
        assert_eq!(
            out,
            "SELECT `ts`, CAST(`val` AS VARCHAR) AS `val`, `memo` FROM `t1` limit 10"
        );
    }

    #[test]
    fn explicit_cols() {
        let out = rewrite_sql("SELECT ts, val, memo FROM db1.t1 WHERE ts > NOW-1d").unwrap();
        // 非 DECIMAL 列保留原写法，DECIMAL 列自动 CAST
        assert_eq!(
            out,
            "SELECT ts, CAST(`val` AS VARCHAR) AS `val`, memo FROM `db1`.`t1` WHERE ts > NOW-1d"
        );
    }

    #[test]
    fn qualified_and_alias() {
        let out = rewrite_sql("select a.val, a.ts from t1 a order by a.ts desc").unwrap();
        assert_eq!(
            out,
            "SELECT CAST(`val` AS VARCHAR) AS `val`, `a`.`ts` FROM `t1` `a` order by a.ts desc"
                .replace("`a`.`ts`", "a.ts")
        );
    }

    #[test]
    fn no_decimal_no_rewrite() {
        // 无 DECIMAL 时返回 None（保持原样）
        let p = parse_simple_select("select * from t1").unwrap();
        assert!(rewrite("select * from t1", &p, &[], None).is_none());
    }

    #[test]
    fn complex_sql_rejected() {
        assert!(parse_simple_select("select a from t1 join t2 on a.id=b.id").is_none());
        assert!(parse_simple_select(
            "select (select max(x) from t2) from t1"
        )
        .is_none());
        assert!(parse_simple_select("select * from t1, t2").is_none());
        assert!(parse_simple_select("insert into t1 values (1)").is_none());
        assert!(parse_simple_select(
            "select * from t1 union select * from t2"
        )
        .is_none());
    }

    #[test]
    fn expr_referencing_decimal_rejected() {
        // 表达式引用了 DECIMAL 列 → 无法安全改写
        assert!(rewrite_sql("select val + 1 from t1").is_none());
        assert!(rewrite_sql("select avg(val) from t1").is_none());
    }

    #[test]
    fn expr_without_decimal_kept() {
        let out = rewrite_sql("select count(*), val from t1").unwrap();
        assert!(out.contains("count(*)"));
        assert!(out.contains("CAST(`val` AS VARCHAR) AS `val`"));
    }

    #[test]
    fn quoted_ident_and_string() {
        // 字符串里出现 from 关键字不应干扰
        let out = rewrite_sql("select `val`, memo from `t1` where memo = 'from x'").unwrap();
        assert!(out.contains("CAST(`val` AS VARCHAR) AS `val`"));
        assert!(out.contains("'from x'"));
    }
}
