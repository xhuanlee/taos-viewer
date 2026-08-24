import { invoke } from "@tauri-apps/api/core";
import type {
  ColumnMeta,
  ConnectionConfig,
  DatabaseInfo,
  QueryResult,
  ServerInfo,
  TableBrief,
} from "@/types";

export function loadConnections(): Promise<ConnectionConfig[]> {
  return invoke("load_connections");
}

export function saveConnections(configs: ConnectionConfig[]): Promise<void> {
  return invoke("save_connections", { configs });
}

export function connect(config: ConnectionConfig): Promise<ServerInfo> {
  return invoke("connect", { config });
}

export function disconnect(connId: string): Promise<void> {
  return invoke("disconnect", { connId });
}

export function testConnection(config: ConnectionConfig): Promise<ServerInfo> {
  return invoke("test_connection", { config });
}

export function executeBatch(params: {
  connId: string;
  db?: string | null;
  sqls: string[];
  maxRows?: number;
}): Promise<QueryResult[]> {
  return invoke("execute_batch", params);
}

export function listDatabases(connId: string): Promise<DatabaseInfo[]> {
  return invoke("list_databases", { connId });
}

export function listTables(connId: string, db: string): Promise<TableBrief[]> {
  return invoke("list_tables", { connId, db });
}

export function describeTable(
  connId: string,
  db: string,
  table: string
): Promise<ColumnMeta[]> {
  return invoke("describe_table", { connId, db, table });
}

export function showCreateTable(
  connId: string,
  db: string,
  table: string,
  kind: string
): Promise<string> {
  return invoke("show_create_table", { connId, db, table, kind });
}
