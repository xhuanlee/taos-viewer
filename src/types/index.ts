export interface ConnectionConfig {
  id: string;
  name: string;
  host: string;
  port: number;
  user: string;
  password: string;
  database?: string | null;
}

export interface ServerInfo {
  version: string;
  serverTime: string;
}

export interface QueryField {
  name: string;
  ty: string;
}

export interface QueryResult {
  sql: string;
  fields: QueryField[];
  rows: unknown[][];
  elapsedMs: number;
  affected: number | null;
  truncated: boolean;
}

export interface DatabaseInfo {
  name: string;
  tables: number;
  precision: string;
}

export interface TableBrief {
  name: string;
  /** stable | table | view */
  kind: string;
}

export interface ColumnMeta {
  name: string;
  ty: string;
  length: number;
  note: string;
}
