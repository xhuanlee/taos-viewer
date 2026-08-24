/**
 * Split a SQL script into individual statements by `;`,
 * respecting quoted strings, backtick identifiers and comments.
 */
export function splitSqlStatements(input: string): string[] {
  const stmts: string[] = [];
  let current = "";
  let i = 0;
  const n = input.length;
  let inSingle = false;
  let inDouble = false;
  let inBacktick = false;
  let inLineComment = false;
  let inBlockComment = false;

  while (i < n) {
    const c = input[i];
    const next = i + 1 < n ? input[i + 1] : "";

    if (inLineComment) {
      current += c;
      if (c === "\n") inLineComment = false;
      i++;
      continue;
    }
    if (inBlockComment) {
      if (c === "*" && next === "/") {
        current += "*/";
        i += 2;
        inBlockComment = false;
      } else {
        current += c;
        i++;
      }
      continue;
    }
    if (inSingle) {
      if (c === "\\" && next) {
        current += c + next;
        i += 2;
        continue;
      }
      current += c;
      if (c === "'") inSingle = false;
      i++;
      continue;
    }
    if (inDouble) {
      if (c === "\\" && next) {
        current += c + next;
        i += 2;
        continue;
      }
      current += c;
      if (c === '"') inDouble = false;
      i++;
      continue;
    }
    if (inBacktick) {
      current += c;
      if (c === "`") inBacktick = false;
      i++;
      continue;
    }

    if (c === "-" && next === "-") {
      inLineComment = true;
      current += c;
      i++;
      continue;
    }
    if (c === "#") {
      inLineComment = true;
      current += c;
      i++;
      continue;
    }
    if (c === "/" && next === "*") {
      inBlockComment = true;
      current += c;
      i++;
      continue;
    }
    if (c === "'") {
      inSingle = true;
      current += c;
      i++;
      continue;
    }
    if (c === '"') {
      inDouble = true;
      current += c;
      i++;
      continue;
    }
    if (c === "`") {
      inBacktick = true;
      current += c;
      i++;
      continue;
    }
    if (c === ";") {
      const trimmed = current.trim();
      if (trimmed) stmts.push(trimmed);
      current = "";
      i++;
      continue;
    }
    current += c;
    i++;
  }

  const trimmed = current.trim();
  if (trimmed) stmts.push(trimmed);
  return stmts;
}

export function quoteIdent(name: string): string {
  return "`" + name.replace(/`/g, "``") + "`";
}

export function firstLine(sql: string, max = 60): string {
  const line = sql.split("\n").find((l) => l.trim().length > 0) ?? "";
  return line.length > max ? line.slice(0, max) + "…" : line;
}
