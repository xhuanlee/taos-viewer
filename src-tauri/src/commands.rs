use std::sync::Arc;
use std::time::Instant;

use futures_util::StreamExt;
use serde_json::{json, Value as Json};
use tauri::State;
use taos::{AsyncFetchable, AsyncQueryable, AsyncTBuilder, Precision, Taos, Value};

use crate::error::{Error, Result};
use crate::models::*;
use crate::state::{AppState, ConnectionHandle};

// ---------- helpers ----------

fn quote_ident(s: &str) -> String {
    format!("`{}`", s.replace('`', "``"))
}

fn build_dsn(c: &ConnectionConfig) -> String {
    format!(
        "ws://{}:{}@{}:{}",
        urlencoding::encode(&c.user),
        urlencoding::encode(&c.password),
        c.host.trim(),
        c.port
    )
}

fn float_json(x: f64) -> Json {
    if x.is_finite() {
        json!(x)
    } else if x.is_nan() {
        json!("NaN")
    } else if x > 0.0 {
        json!("Infinity")
    } else {
        json!("-Infinity")
    }
}

fn value_to_json(v: &Value) -> Json {
    match v {
        Value::Null(_) => Json::Null,
        Value::Bool(b) => json!(b),
        Value::TinyInt(x) => json!(x),
        Value::SmallInt(x) => json!(x),
        Value::Int(x) => json!(x),
        Value::BigInt(x) => json!(x),
        Value::UTinyInt(x) => json!(x),
        Value::USmallInt(x) => json!(x),
        Value::UInt(x) => json!(x),
        Value::UBigInt(x) => json!(x),
        Value::Float(x) => float_json(*x as f64),
        Value::Double(x) => float_json(*x),
        Value::VarChar(s) => json!(s),
        Value::NChar(s) => json!(s),
        Value::Timestamp(ts) => {
            let dt = ts.to_datetime_with_tz();
            let s = match ts.precision() {
                Precision::Millisecond => dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
                Precision::Microsecond => dt.format("%Y-%m-%d %H:%M:%S%.6f").to_string(),
                Precision::Nanosecond => dt.format("%Y-%m-%d %H:%M:%S%.9f").to_string(),
            };
            Json::String(s)
        }
        Value::Json(j) => j.clone(),
        other => Json::String(format!("{other:?}")),
    }
}

fn json_to_string(v: Option<&Json>) -> String {
    match v {
        Some(Json::String(s)) => s.clone(),
        Some(Json::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn json_to_u64(v: Option<&Json>) -> Option<u64> {
    match v {
        Some(Json::Number(n)) => n.as_u64(),
        Some(Json::String(s)) => s.parse().ok(),
        _ => None,
    }
}

async fn connect_impl(c: &ConnectionConfig) -> Result<Taos> {
    let taos = taos::TaosBuilder::from_dsn(build_dsn(c))?
        .build()
        .await?;
    Ok(taos)
}

/// Run one SQL statement and collect fields / rows into a QueryResult.
async fn query_to_result(taos: &Taos, sql: String, max_rows: u64) -> Result<QueryResult> {
    let start = Instant::now();
    let mut result = match taos.query(&sql).await {
        Ok(r) => r,
        // DDL/USE 等语句无结果集（taosAdapter 返回 "result is nil"），改用 exec
        Err(source) if source.to_string().contains("result is nil") => {
            let affected = taos.exec(&sql).await.map_err(|source2| Error::SqlFailed {
                sql: sql.clone(),
                source: source2,
            })?;
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            return Ok(QueryResult {
                sql,
                fields: Vec::new(),
                rows: Vec::new(),
                elapsed_ms,
                affected: if affected > 0 {
                    Some(affected as u64)
                } else {
                    None
                },
                truncated: false,
            });
        }
        Err(source) => {
            return Err(Error::SqlFailed { sql, source });
        }
    };
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    // 无字段 = 无结果集语句（DDL / USE / INSERT 等）。
    // 此时结果集 id 无对应数据块，继续迭代行会对不存在的结果集发起 fetch，
    // 得到 "result is nil" 错误 —— 直接返回受影响行数即可。
    if result.fields().is_empty() {
        let affected = result.affected_rows64();
        return Ok(QueryResult {
            sql,
            fields: Vec::new(),
            rows: Vec::new(),
            elapsed_ms,
            affected: if affected > 0 {
                Some(affected as u64)
            } else {
                None
            },
            truncated: false,
        });
    }

    let fields: Vec<QueryField> = result
        .fields()
        .iter()
        .map(|f| QueryField {
            name: f.name().to_string(),
            ty: f.ty().to_string(),
        })
        .collect();
    let affected = result.affected_rows64();

    let mut rows_data: Vec<Vec<Json>> = Vec::new();
    let mut truncated = false;

    {
        let mut rows = result.rows();
        while let Some(item) = rows.next().await {
            let row = item.map_err(Error::Taos)?;
            let values = row.into_values();
            let json_row: Vec<Json> = values.iter().map(value_to_json).collect();
            rows_data.push(json_row);
            if rows_data.len() as u64 >= max_rows {
                truncated = true;
                break;
            }
        }
    }

    Ok(QueryResult {
        sql,
        fields,
        rows: rows_data,
        elapsed_ms,
        affected: if affected > 0 {
            Some(affected as u64)
        } else {
            None
        },
        truncated,
    })
}

// ---------- connection management ----------

#[tauri::command]
pub fn load_connections(state: State<'_, AppState>) -> Vec<ConnectionConfig> {
    state.load_configs()
}

#[tauri::command]
pub fn save_connections(state: State<'_, AppState>, configs: Vec<ConnectionConfig>) -> Result<()> {
    state.save_configs(&configs)
}

#[tauri::command]
pub async fn test_connection(config: ConnectionConfig) -> Result<ServerInfo> {
    let taos = connect_impl(&config).await?;
    fetch_server_info(&taos).await
}

#[tauri::command]
pub async fn connect(state: State<'_, AppState>, config: ConnectionConfig) -> Result<ServerInfo> {
    let taos = Arc::new(connect_impl(&config).await?);
    let info = fetch_server_info(&taos).await?;
    state
        .connections
        .lock()
        .unwrap()
        .insert(config.id.clone(), new_handle(taos, config));
    Ok(info)
}

fn new_handle(taos: Arc<Taos>, config: ConnectionConfig) -> Arc<ConnectionHandle> {
    Arc::new(ConnectionHandle {
        taos,
        lock: tokio::sync::Mutex::new(()),
        config,
    })
}

/// 连接疑似损坏（taos-ws 后台任务 panic 导致查询永久挂起）时，
/// 用保存的配置重建连接。
async fn reconnect(state: &State<'_, AppState>, conn_id: &str) -> Result<Arc<ConnectionHandle>> {
    let handle = state.get_conn(conn_id)?;
    let config = handle.config.clone();
    let taos = Arc::new(connect_impl(&config).await?);
    let fresh = new_handle(taos, config);
    state
        .connections
        .lock()
        .unwrap()
        .insert(conn_id.to_string(), fresh.clone());
    Ok(fresh)
}

#[tauri::command]
pub fn disconnect(state: State<'_, AppState>, conn_id: String) {
    state.connections.lock().unwrap().remove(&conn_id);
}

async fn fetch_server_info(taos: &Taos) -> Result<ServerInfo> {
    // TDengine 3.x 与较新的 2.x 支持 SERVER_VERSION() 函数
    if let Ok(r) = query_to_result(
        taos,
        "SELECT SERVER_VERSION() AS version, NOW() AS server_time".to_string(),
        10,
    )
    .await
    {
        if let Some(row) = r.rows.first() {
            return Ok(ServerInfo {
                version: json_to_string(row.first()),
                server_time: json_to_string(row.get(1)),
            });
        }
    }
    // 老版本 2.x 不支持该函数，用 SHOW DATABASES 验证连接可用即可
    query_to_result(taos, "SHOW DATABASES".to_string(), 10).await?;
    Ok(ServerInfo {
        version: "2.x".into(),
        server_time: String::new(),
    })
}

// ---------- generic SQL execution ----------

use std::future::Future;
use std::time::Duration;

const QUERY_TIMEOUT: Duration = Duration::from_secs(30);

/// 带保护的查询执行：
/// 1. 查询在独立任务中运行（连接锁在任务内获取），
///    保证同连接串行执行，避免 USE 与查询交错、规避 taos-ws 并发竞态；
/// 2. taos-query 解析 DECIMAL 等类型时会 panic，独立任务可将 panic 转为错误；
/// 3. 部分服务端的 DECIMAL 二进制解析会让 taos-ws 后台任务 panic，
///    查询将永久挂起 —— 超时后自动重建连接并返回错误。
async fn run_guarded<T, F, Fut>(state: &State<'_, AppState>, conn_id: &str, f: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(std::sync::Arc<ConnectionHandle>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T>> + Send + 'static,
{
    let handle = state.get_conn(conn_id)?;
    let join = tokio::task::spawn(async move { f(handle).await });
    match tokio::time::timeout(QUERY_TIMEOUT, join).await {
        Ok(Ok(v)) => v,
        Ok(Err(join_err)) if join_err.is_panic() => {
            // panic 会污染连接内部状态（后续 DDL 报错甚至连接挂死），必须重建
            let _ = reconnect(state, conn_id).await;
            Err(Error::Message(
                "查询失败：结果集中包含当前驱动暂不支持的数据类型（如 DECIMAL）。\
                 请在 SQL 中对相关列使用 CAST(列 AS VARCHAR) 转换后重试。\
                 连接已自动恢复"
                    .into(),
            ))
        }
        Ok(Err(_)) => {
            let _ = reconnect(state, conn_id).await;
            Err(Error::Message("查询任务异常终止，连接已自动恢复".into()))
        }
        Err(_) => {
            // 查询挂起：连接后台任务已死亡，自动重建
            let _ = reconnect(state, conn_id).await;
            Err(Error::Message(
                "查询超时，连接已自动恢复。若结果包含 DECIMAL 等特殊类型列，\
                 请使用 CAST(列 AS VARCHAR) 转换后重试"
                    .into(),
            ))
        }
    }
}

#[tauri::command]
pub async fn execute_batch(
    state: State<'_, AppState>,
    conn_id: String,
    db: Option<String>,
    sqls: Vec<String>,
    max_rows: Option<u64>,
) -> Result<Vec<QueryResult>> {
    let max_rows = max_rows.unwrap_or(10_000).clamp(1, 200_000);
    run_guarded(&state, &conn_id, move |handle| async move {
        let _guard = handle.lock.lock().await;
        let taos = &handle.taos;

        if let Some(db) = &db {
            taos.exec(format!("USE {}", quote_ident(db))).await?;
        }

        let mut results = Vec::with_capacity(sqls.len());
        for sql in sqls {
            let result = query_to_result(taos, sql, max_rows).await?;
            results.push(result);
        }
        Ok(results)
    })
    .await
}

// ---------- metadata browsing ----------

#[tauri::command]
pub async fn list_databases(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<Vec<DatabaseInfo>> {
    run_guarded(&state, &conn_id, |handle| async move {
        let _guard = handle.lock.lock().await;
        let taos = &handle.taos;
        // SHOW DATABASES 兼容 TDengine 2.x / 3.x 全系版本
        // （information_schema.ins_databases 仅 3.x 支持）
        let r = query_to_result(taos, "SHOW DATABASES".to_string(), 10_000).await?;

        let idx = |name: &str| {
            r.fields
                .iter()
                .position(|f| f.name.eq_ignore_ascii_case(name))
        };

        let mut list = Vec::new();
        for row in &r.rows {
            let pick = |name: &str| idx(name).and_then(|i| row.get(i));
            let name = json_to_string(pick("name"));
            // 过滤系统库：2.x 为 log，3.x 为 information_schema / performance_schema
            if matches!(
                name.to_lowercase().as_str(),
                "log" | "information_schema" | "performance_schema"
            ) {
                continue;
            }
            list.push(DatabaseInfo {
                name,
                tables: json_to_u64(pick("ntables")).unwrap_or(0),
                precision: json_to_string(pick("precision")),
            });
        }
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(list)
    })
    .await
}

#[tauri::command]
pub async fn list_tables(
    state: State<'_, AppState>,
    conn_id: String,
    db: String,
) -> Result<Vec<TableBrief>> {
    run_guarded(&state, &conn_id, move |handle| async move {
        let _guard = handle.lock.lock().await;
        let taos = &handle.taos;
        let mut list: Vec<TableBrief> = Vec::new();

        // 超级表：SHOW <db>.STABLES 兼容 2.x / 3.x，首列为 stable_name
        let stables = query_to_result(taos, format!("SHOW {}.STABLES", quote_ident(&db)), 200_000)
            .await?;
        for row in &stables.rows {
            let name = json_to_string(row.first());
            if !name.is_empty() {
                list.push(TableBrief {
                    name,
                    kind: "stable".into(),
                });
            }
        }

        // 普通表：SHOW <db>.TABLES 兼容 2.x / 3.x
        // stable_name 列非空的是子表，跳过
        let tables = query_to_result(taos, format!("SHOW {}.TABLES", quote_ident(&db)), 200_000)
            .await?;
        let stable_idx = tables
            .fields
            .iter()
            .position(|f| f.name.eq_ignore_ascii_case("stable_name"));
        for row in &tables.rows {
            let name = json_to_string(row.first());
            let stable = json_to_string(stable_idx.and_then(|i| row.get(i)));
            if name.is_empty() || !stable.is_empty() {
                continue;
            }
            list.push(TableBrief {
                name,
                kind: "table".into(),
            });
        }

        // 视图：仅 3.x 支持，2.x 或无视图权限时忽略错误
        if let Ok(views) = query_to_result(taos, format!("SHOW {}.VIEWS", quote_ident(&db)), 200_000)
            .await
        {
            for row in &views.rows {
                let name = json_to_string(row.first());
                if !name.is_empty() {
                    list.push(TableBrief {
                        name,
                        kind: "view".into(),
                    });
                }
            }
        }

        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(list)
    })
    .await
}

#[tauri::command]
pub async fn describe_table(
    state: State<'_, AppState>,
    conn_id: String,
    db: String,
    table: String,
) -> Result<Vec<ColumnMeta>> {
    run_guarded(&state, &conn_id, move |handle| async move {
        let _guard = handle.lock.lock().await;
        let taos = &handle.taos;
        let sql = format!("DESCRIBE {}.{}", quote_ident(&db), quote_ident(&table));
        let r = query_to_result(taos, sql, 10_000).await?;

        let mut list = Vec::new();
        for row in &r.rows {
            list.push(ColumnMeta {
                name: json_to_string(row.first()),
                ty: json_to_string(row.get(1)),
                length: json_to_u64(row.get(2)).unwrap_or(0),
                note: json_to_string(row.get(3)),
            });
        }
        Ok(list)
    })
    .await
}

#[tauri::command]
pub async fn show_create_table(
    state: State<'_, AppState>,
    conn_id: String,
    db: String,
    table: String,
    kind: String,
) -> Result<String> {
    run_guarded(&state, &conn_id, move |handle| async move {
        let _guard = handle.lock.lock().await;
        let taos = &handle.taos;
        let sql = if kind == "stable" {
            format!(
                "SHOW CREATE STABLE {}.{}",
                quote_ident(&db),
                quote_ident(&table)
            )
        } else {
            format!(
                "SHOW CREATE TABLE {}.{}",
                quote_ident(&db),
                quote_ident(&table)
            )
        };
        let r = query_to_result(taos, sql, 10).await?;
        if let Some(row) = r.rows.first() {
            if row.len() >= 2 {
                return Ok(json_to_string(row.get(1)));
            }
        }
        Err(Error::Message("无法获取建表语句".into()))
    })
    .await
}

// ---------- tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    /// 本地连接：可用环境变量覆盖（TAOS_TEST_LOCAL_HOST / _PORT / _USER / _PASSWORD）
    fn local_config() -> ConnectionConfig {
        ConnectionConfig {
            id: "test-local".into(),
            name: "local".into(),
            host: env_or("TAOS_TEST_LOCAL_HOST", "localhost"),
            port: env_or("TAOS_TEST_LOCAL_PORT", "6041").parse().unwrap(),
            user: env_or("TAOS_TEST_LOCAL_USER", "root"),
            password: env_or("TAOS_TEST_LOCAL_PASSWORD", "taosdata"),
            database: None,
        }
    }

    /// 远程连接：凭据不入库，全部来自环境变量
    /// （TAOS_TEST_REMOTE_HOST / _PORT / _USER / _PASSWORD / _DB），未配置则跳过测试
    fn remote_config() -> Option<ConnectionConfig> {
        let host = std::env::var("TAOS_TEST_REMOTE_HOST").ok()?;
        Some(ConnectionConfig {
            id: "test-remote".into(),
            name: "remote".into(),
            host,
            port: env_or("TAOS_TEST_REMOTE_PORT", "6041").parse().unwrap(),
            user: env_or("TAOS_TEST_REMOTE_USER", "root"),
            password: env_or("TAOS_TEST_REMOTE_PASSWORD", "taosdata"),
            database: None,
        })
    }

    fn env_or(key: &str, default: &str) -> String {
        std::env::var(key).unwrap_or_else(|_| default.to_string())
    }

    /// 模拟 execute_batch 的串行化执行：USE db 后依次执行 sqls
    async fn exec_batch_on(
        taos: &Taos,
        db: Option<&str>,
        sqls: &[String],
        max_rows: u64,
    ) -> Result<Vec<QueryResult>> {
        if let Some(db) = db {
            taos.exec(format!("USE {}", quote_ident(db))).await?;
        }
        let mut results = Vec::with_capacity(sqls.len());
        for sql in sqls {
            results.push(query_to_result(taos, sql.clone(), max_rows).await?);
        }
        Ok(results)
    }

    /// 模拟 TableDataTab 的列选择构建：DECIMAL 列 CAST 为 VARCHAR
    fn build_select_list(cols: &[ColumnMeta]) -> String {
        cols.iter()
            .map(|c| {
                if c.ty.to_uppercase().starts_with("DECIMAL") {
                    format!(
                        "CAST({} AS VARCHAR) AS {}",
                        quote_ident(&c.name),
                        quote_ident(&c.name)
                    )
                } else {
                    quote_ident(&c.name)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn describe_to_cols(r: &QueryResult) -> Vec<ColumnMeta> {
        r.rows
            .iter()
            .map(|row| ColumnMeta {
                name: json_to_string(row.first()),
                ty: json_to_string(row.get(1)),
                length: json_to_u64(row.get(2)).unwrap_or(0),
                note: json_to_string(row.get(3)),
            })
            .collect()
    }

    /// 完整读路径功能套件：连接 → 库列表 → 表列表 → DESCRIBE → DDL → 数据浏览 → 查询
    async fn run_read_suite(config: &ConnectionConfig, db: &str) {
        let taos = connect_impl(config).await.expect("连接失败");

        // 1. 服务器信息
        let info = fetch_server_info(&taos).await.expect("获取服务器信息失败");
        assert!(!info.version.is_empty(), "版本号不应为空");
        println!("[{}] server version = {}", config.name, info.version);

        // 2. 数据库列表
        let dbs = exec_batch_on(&taos, None, &["SHOW DATABASES".to_string()], 10_000)
            .await
            .expect("SHOW DATABASES 失败");
        assert!(!dbs[0].rows.is_empty(), "数据库列表不应为空");
        let names: Vec<String> = dbs[0]
            .rows
            .iter()
            .filter_map(|r| r.first().map(|v| json_to_string(Some(v))))
            .collect();
        assert!(
            names.iter().any(|n| n.eq_ignore_ascii_case(db)),
            "应包含库 {db}，实际：{names:?}"
        );
        println!("[{}] databases = {:?}", config.name, names);

        // 3. 表列表（超级表 + 普通表）
        let tables = exec_batch_on(
            &taos,
            Some(db),
            &[
                format!("SHOW {}.STABLES", quote_ident(db)),
                format!("SHOW {}.TABLES", quote_ident(db)),
            ],
            200_000,
        )
        .await
        .expect("SHOW TABLES 失败");
        println!(
            "[{}] {} stables, {} tables",
            config.name,
            tables[0].rows.len(),
            tables[1].rows.len()
        );

        // 4. 选一个对象做 DESCRIBE / SHOW CREATE / 数据浏览
        let (target, kind) = if !tables[0].rows.is_empty() {
            (
                json_to_string(tables[0].rows[0].first()),
                "stable",
            )
        } else {
            (
                json_to_string(tables[1].rows[0].first()),
                "table",
            )
        };
        assert!(!target.is_empty(), "浏览对象名不应为空");

        // 5. DESCRIBE（表设计器字段列表）
        let desc = exec_batch_on(
            &taos,
            Some(db),
            &[format!(
                "DESCRIBE {}.{}",
                quote_ident(db),
                quote_ident(&target)
            )],
            10_000,
        )
        .await
        .expect("DESCRIBE 失败");
        let cols = describe_to_cols(&desc[0]);
        assert!(!cols.is_empty(), "字段列表不应为空");
        println!("[{}] {} columns: {:?}", config.name, target, cols.iter().map(|c| c.ty.clone()).collect::<Vec<_>>());

        // 6. SHOW CREATE（表设计器 DDL）
        let ddl_sql = if kind == "stable" {
            format!(
                "SHOW CREATE STABLE {}.{}",
                quote_ident(db),
                quote_ident(&target)
            )
        } else {
            format!(
                "SHOW CREATE TABLE {}.{}",
                quote_ident(db),
                quote_ident(&target)
            )
        };
        let ddl = exec_batch_on(&taos, Some(db), &[ddl_sql], 10)
            .await
            .expect("SHOW CREATE 失败");
        let ddl_text = json_to_string(ddl[0].rows[0].get(1));
        assert!(
            ddl_text.to_uppercase().contains("CREATE"),
            "DDL 应包含 CREATE：{ddl_text}"
        );
        println!("[{}] DDL ok ({} chars)", config.name, ddl_text.len());

        // 7. 数据浏览模式（TableDataTab 的批量查询：COUNT + 分页 SELECT，DECIMAL 已 CAST）
        let select_list = build_select_list(&cols);
        let full = format!("{}.{}", quote_ident(db), quote_ident(&target));
        let results = exec_batch_on(
            &taos,
            Some(db),
            &[
                format!("SELECT COUNT(*) FROM {full}"),
                format!("SELECT {select_list} FROM {full} LIMIT 100 OFFSET 0"),
            ],
            100,
        )
        .await
        .expect("数据浏览查询失败（DECIMAL CAST 后不应再挂起/报错）");
        let total = json_to_u64(results[0].rows[0].first()).unwrap_or(0);
        assert_eq!(results[1].fields.len(), cols.len(), "结果列数应与字段数一致");
        assert!(
            results[1].rows.len() as u64 <= 100,
            "返回行数不应超过 LIMIT"
        );
        println!(
            "[{}] data browse ok: total = {}, page rows = {}",
            config.name,
            total,
            results[1].rows.len()
        );

        // 8. 普通查询（QueryTab 模式）：SELECT 应返回字段和数据行
        if total > 0 {
            assert!(
                !results[1].rows.is_empty(),
                "有数据时查询结果必须返回行（此前 bug：只见表头不见数据）"
            );
            // 抽查一行：每个字段都有值（NULL 或具体值），行结构与字段数一致
            assert_eq!(results[1].rows[0].len(), cols.len());
            println!(
                "[{}] sample row: {:?}",
                config.name,
                results[1].rows[0]
            );
        }
    }

    /// DDL 生命周期套件：建库 → 建超级表（含 DECIMAL）→ 建子表 → 插入 →
    /// 浏览（DECIMAL CAST）→ ALTER 加列 → 验证 → 清理
    async fn run_ddl_suite(config: &ConnectionConfig) {
        let taos = connect_impl(config).await.expect("连接失败");
        let db = "taos_viewer_test";
        let _ = taos.exec(format!("DROP DATABASE IF EXISTS {}", quote_ident(db))).await;

        // 建库 + 建表
        let sqls = vec![
            format!("CREATE DATABASE {}", quote_ident(db)),
            format!(
                "CREATE STABLE {}.metric (ts TIMESTAMP, val DOUBLE, price DECIMAL(10,2), memo VARCHAR(64)) TAGS (sid INT)",
                quote_ident(db)
            ),
            format!(
                "CREATE TABLE {}.m1 USING {}.metric TAGS (1)",
                quote_ident(db),
                quote_ident(db)
            ),
        ];
        exec_batch_on(&taos, None, &sqls, 100)
            .await
            .expect("建库/建表失败");

        // 插入数据（含 DECIMAL 值）
        let inserts = vec![
            "INSERT INTO taos_viewer_test.m1 VALUES (NOW-10s, 1.5, 10.25, 'a')".to_string(),
            "INSERT INTO taos_viewer_test.m1 VALUES (NOW-5s, 2.5, 20.5, 'b')".to_string(),
            "INSERT INTO taos_viewer_test.m1 VALUES (NOW, 3.5, 30.75, 'c')".to_string(),
        ];
        let res = exec_batch_on(&taos, Some(db), &inserts, 100)
            .await
            .expect("INSERT 失败");
        assert!(res.iter().all(|r| r.affected.unwrap_or(0) > 0), "INSERT 应影响行数");

        // 浏览数据：DESCRIBE → DECIMAL CAST → COUNT + SELECT（核心 bug 场景）
        let desc = exec_batch_on(
            &taos,
            Some(db),
            &["DESCRIBE taos_viewer_test.m1".to_string()],
            100,
        )
        .await
        .expect("DESCRIBE 失败");
        let cols = describe_to_cols(&desc[0]);
        assert!(cols.iter().any(|c| c.ty.starts_with("DECIMAL")), "应包含 DECIMAL 列");

        let select_list = build_select_list(&cols);
        let results = exec_batch_on(
            &taos,
            Some(db),
            &[
                "SELECT COUNT(*) FROM taos_viewer_test.m1".to_string(),
                format!("SELECT {select_list} FROM taos_viewer_test.m1 LIMIT 100 OFFSET 0"),
            ],
            100,
        )
        .await
        .expect("含 DECIMAL 的表浏览失败");
        let total = json_to_u64(results[0].rows[0].first()).unwrap_or(0);
        assert_eq!(total, 3, "应有 3 行数据");
        assert_eq!(results[1].rows.len(), 3, "分页查询应返回 3 行");
        // DECIMAL CAST 后应能取到字符串数值
        let price_idx = cols.iter().position(|c| c.name == "price").unwrap();
        let prices: Vec<String> = results[1]
            .rows
            .iter()
            .map(|r| json_to_string(r.get(price_idx)))
            .collect();
        assert!(
            prices.iter().all(|p| !p.is_empty()),
            "DECIMAL 列 CAST 后应有值：{prices:?}"
        );
        println!("[{}] DECIMAL browse ok: prices = {:?}", config.name, prices);

        // ALTER 加列（表设计器修改字段）。
        // 注意：子表不允许 ADD COLUMN，须对超级表执行（与应用中 stable → ALTER STABLE 一致）
        exec_batch_on(
            &taos,
            Some(db),
            &[format!(
                "ALTER STABLE {}.{} ADD COLUMN extra VARCHAR(32)",
                quote_ident(db),
                quote_ident("metric")
            )],
            100,
        )
        .await
        .expect("ALTER STABLE 失败");

        let desc2 = exec_batch_on(
            &taos,
            Some(db),
            &["DESCRIBE taos_viewer_test.m1".to_string()],
            100,
        )
        .await
        .expect("ALTER 后 DESCRIBE 失败");
        let cols2 = describe_to_cols(&desc2[0]);
        assert!(
            cols2.iter().any(|c| c.name == "extra"),
            "新列应出现在字段列表"
        );

        // 普通表上的 ALTER TABLE（应用对普通表走 ALTER TABLE）
        exec_batch_on(
            &taos,
            Some(db),
            &[
                "CREATE TABLE taos_viewer_test.plain (ts TIMESTAMP, v INT)".to_string(),
                "ALTER TABLE taos_viewer_test.plain ADD COLUMN note VARCHAR(16)".to_string(),
            ],
            100,
        )
        .await
        .expect("普通表建表/ALTER 失败");

        // SHOW CREATE 验证
        let ddl = exec_batch_on(
            &taos,
            Some(db),
            &["SHOW CREATE TABLE taos_viewer_test.m1".to_string()],
            10,
        )
        .await
        .expect("SHOW CREATE 失败");
        assert!(json_to_string(ddl[0].rows[0].get(1)).contains("CREATE"));

        // 清理
        taos.exec(format!("DROP DATABASE IF EXISTS {}", quote_ident(db)))
            .await
            .expect("清理失败");
        println!("[{}] DDL suite passed", config.name);
    }

    #[tokio::test]
    async fn e2e_read_local() {
        let db = env_or("TAOS_TEST_LOCAL_DB", "tsdb_taihe");
        run_read_suite(&local_config(), &db).await;
    }

    /// 裸 SELECT DECIMAL 列可能使 taos-query panic 或挂起连接。
    /// 验证：独立任务执行 + panic 捕获 + 连接重建后可继续查询
    /// （run_guarded 的核心保护机制）。
    #[tokio::test]
    async fn decimal_panic_recovery_local() {
        let config = local_config();
        let taos = Arc::new(connect_impl(&config).await.expect("连接失败"));
        let db = "taos_viewer_dectest";
        let _ = taos.exec(format!("DROP DATABASE IF EXISTS {}", quote_ident(db))).await;
        exec_batch_on(
            &taos,
            None,
            &[
                format!("CREATE DATABASE {}", quote_ident(db)),
                "CREATE TABLE taos_viewer_dectest.d1 (ts TIMESTAMP, price DECIMAL(10,2))".to_string(),
                "INSERT INTO taos_viewer_dectest.d1 VALUES (NOW, 12.34)".to_string(),
            ],
            100,
        )
        .await
        .expect("准备数据失败");

        // 在独立任务中执行裸 DECIMAL 查询（模拟 run_guarded 的 spawn）。
        // 已知驱动限制：taos-ws 后台读取任务在反序列化 DECIMAL 字段类型时会
        // panic（"unknown data type"），此后查询永久挂起 —— run_guarded 通过
        // 超时 + 自动重连兜底，表数据浏览则通过自动 CAST 规避此问题。
        let t = taos.clone();
        let join = tokio::task::spawn(async move {
            query_to_result(&t, "SELECT price FROM taos_viewer_dectest.d1".to_string(), 100).await
        });
        let outcome = tokio::time::timeout(Duration::from_secs(5), join).await;

        match outcome {
            Ok(Ok(Ok(_))) => {
                // 驱动已支持 DECIMAL：直接通过
                println!("raw DECIMAL select ok (driver supports it)");
            }
            Ok(Ok(Err(e))) => {
                // 返回错误：可接受的失败形态
                println!("raw DECIMAL select error (acceptable): {e}");
            }
            Ok(Err(join_err)) => {
                assert!(join_err.is_panic(), "应为 panic");
                println!("raw DECIMAL select panicked (captured by task boundary)");
            }
            Err(_) => {
                // 挂起直到超时：已知驱动限制，run_guarded 会超时重连
                println!("raw DECIMAL select hung until timeout (known driver limitation)");
            }
        }

        // 无论哪种情况，重建连接后查询必须能继续工作
        let fresh = Arc::new(connect_impl(&config).await.expect("重连失败"));
        let res = query_to_result(
            &fresh,
            "SELECT CAST(price AS VARCHAR) AS price FROM taos_viewer_dectest.d1".to_string(),
            100,
        )
        .await
        .expect("重连后 CAST 查询失败");
        assert_eq!(res.rows.len(), 1);
        let v = json_to_string(res.rows[0].first());
        assert_eq!(v, "12.34");
        println!("reconnect + CAST query ok: price = {v}");

        let _ = fresh
            .exec(format!("DROP DATABASE IF EXISTS {}", quote_ident(db)))
            .await;
    }

    #[tokio::test]
    async fn e2e_read_remote() {
        let Some(config) = remote_config() else {
            println!("未配置 TAOS_TEST_REMOTE_* 环境变量，跳过远程读测试");
            return;
        };
        let db = env_or("TAOS_TEST_REMOTE_DB", "test");
        run_read_suite(&config, &db).await;
    }

    #[tokio::test]
    async fn ddl_roundtrip_local() {
        run_ddl_suite(&local_config()).await;
    }

    #[tokio::test]
    async fn ddl_roundtrip_remote() {
        let Some(config) = remote_config() else {
            println!("未配置 TAOS_TEST_REMOTE_* 环境变量，跳过远程 DDL 测试");
            return;
        };
        run_ddl_suite(&config).await;
    }
}
