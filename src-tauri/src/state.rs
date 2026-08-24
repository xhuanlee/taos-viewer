use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use taos::Taos;

use crate::error::{Error, Result};
use crate::models::ConnectionConfig;

/// 一个连接：TDengine WS 客户端 + 查询互斥锁。
///
/// taos-ws 在旧版 taosAdapter（TDengine 2.x / 3.0.x）上的并发查询存在竞态，
/// 可能触发内部 panic；同时 `USE db` 与查询交错会导致上下文错乱。
/// 因此对每个连接串行执行所有查询，保证稳定性。
pub struct ConnectionHandle {
    pub taos: Arc<Taos>,
    pub lock: tokio::sync::Mutex<()>,
    /// 重建连接所需的配置（连接因 panic 损坏后自动恢复用）
    pub config: ConnectionConfig,
}

pub struct AppState {
    pub config_path: PathBuf,
    pub connections: Mutex<HashMap<String, Arc<ConnectionHandle>>>,
}

impl AppState {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            connections: Mutex::new(HashMap::new()),
        }
    }

    pub fn load_configs(&self) -> Vec<ConnectionConfig> {
        match std::fs::read_to_string(&self.config_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    pub fn save_configs(&self, configs: &[ConnectionConfig]) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(configs)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }

    pub fn get_conn(&self, id: &str) -> Result<Arc<ConnectionHandle>> {
        self.connections
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or(Error::NotConnected)
    }
}
