// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A data-only MySQL dump reader. Nothing is executed, including version comments.
//! Literals remain bytes; charset decoding belongs to a reviewed mapping profile.
//! Errors identify syntax locations and codes, never source values.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

const MAX_BYTES: usize = 128 * 1024 * 1024;
const MAX_TOKENS: usize = 4_000_000;
const MAX_ROWS: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlValue {
    Null,
    Bytes(Vec<u8>),
}

#[derive(Debug, Default)]
pub struct SqlTable {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<SqlValue>>,
    pub charset: Option<String>,
    pub column_charsets: BTreeMap<String, String>,
    pub unique_keys: Vec<Vec<String>>,
    pub primary_key: Vec<String>,
}

#[derive(Debug, Default)]
pub struct SqlDump {
    pub tables: BTreeMap<String, SqlTable>,
    /// Views are recorded as names only. Conversion uses base tables explicitly.
    pub views: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpError {
    pub line: usize,
    pub code: &'static str,
}
impl fmt::Display for DumpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at SQL line {}", self.code, self.line)
    }
}
impl std::error::Error for DumpError {}

#[derive(Debug, PartialEq)]
enum Kind {
    Word(Vec<u8>),
    Ident(String),
    Literal(Vec<u8>),
    Symbol(u8),
}
#[derive(Debug)]
struct Token {
    kind: Kind,
    line: usize,
}
impl Token {
    fn word(&self, s: &str) -> bool {
        matches!(&self.kind, Kind::Word(b) if b.eq_ignore_ascii_case(s.as_bytes()))
    }
    fn symbol(&self, c: u8) -> bool {
        self.kind == Kind::Symbol(c)
    }
    fn identifier(&self) -> Result<String, DumpError> {
        match &self.kind {
            Kind::Ident(s) => Ok(s.clone()),
            Kind::Word(b)
                if b.iter()
                    .all(|c| c.is_ascii_alphanumeric() || *c == b'_') =>
            {
                String::from_utf8(b.clone())
                    .map_err(|_| self.error("sql.identifier"))
            }
            _ => Err(self.error("sql.identifier")),
        }
    }
    fn error(&self, code: &'static str) -> DumpError {
        DumpError {
            line: self.line,
            code,
        }
    }
}

struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
    line: usize,
    conditional: bool,
}
impl<'a> Lexer<'a> {
    fn tokens(mut self) -> Result<Vec<Token>, DumpError> {
        let mut out = Vec::new();
        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            if c.is_ascii_whitespace() {
                self.bump();
                continue;
            }
            if self.conditional && self.bytes[self.pos..].starts_with(b"*/") {
                self.pos += 2;
                self.conditional = false;
                continue;
            }
            if c == b'#'
                || (self.bytes[self.pos..].starts_with(b"--")
                    && self
                        .bytes
                        .get(self.pos + 2)
                        .is_none_or(u8::is_ascii_whitespace))
            {
                while self.pos < self.bytes.len()
                    && self.bytes[self.pos] != b'\n'
                {
                    self.bump();
                }
                continue;
            }
            if self.bytes[self.pos..].starts_with(b"/*") {
                if self.conditional {
                    return Err(self.error("sql.nested-comment"));
                }
                self.pos += 2;
                if self.bytes.get(self.pos) == Some(&b'!') {
                    self.pos += 1;
                    while self
                        .bytes
                        .get(self.pos)
                        .is_some_and(u8::is_ascii_digit)
                    {
                        self.pos += 1;
                    }
                    self.conditional = true;
                } else {
                    while self.pos < self.bytes.len()
                        && !self.bytes[self.pos..].starts_with(b"*/")
                    {
                        self.bump();
                    }
                    if self.pos == self.bytes.len() {
                        return Err(self.error("sql.unterminated-comment"));
                    }
                    self.pos += 2;
                }
                continue;
            }
            let line = self.line;
            let kind = if c == b'\'' || c == b'"' || c == b'`' {
                let quoted = self.quoted(c)?;
                if c == b'`' {
                    Kind::Ident(
                        String::from_utf8(quoted)
                            .map_err(|_| self.error("sql.identifier"))?,
                    )
                } else {
                    Kind::Literal(quoted)
                }
            } else if c.is_ascii_alphanumeric()
                || c == b'_'
                || c == b'-'
                || c == b'.'
            {
                let start = self.pos;
                self.pos += 1;
                while self.bytes.get(self.pos).is_some_and(|b| {
                    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.')
                }) {
                    self.pos += 1;
                }
                Kind::Word(self.bytes[start..self.pos].to_vec())
            } else {
                self.pos += 1;
                Kind::Symbol(c)
            };
            out.push(Token { kind, line });
            if out.len() > MAX_TOKENS {
                return Err(self.error("sql.limit"));
            }
        }
        if self.conditional {
            return Err(self.error("sql.unterminated-comment"));
        }
        Ok(out)
    }
    fn bump(&mut self) {
        if self.bytes[self.pos] == b'\n' {
            self.line += 1;
        }
        self.pos += 1;
    }
    fn error(&self, code: &'static str) -> DumpError {
        DumpError {
            line: self.line,
            code,
        }
    }
    fn quoted(&mut self, quote: u8) -> Result<Vec<u8>, DumpError> {
        self.pos += 1;
        let mut value = Vec::new();
        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            self.bump();
            if c == quote {
                if self.bytes.get(self.pos) == Some(&quote) {
                    self.pos += 1;
                    value.push(quote);
                    continue;
                }
                return Ok(value);
            }
            if c == b'\\' && quote != b'`' {
                let Some(&escaped) = self.bytes.get(self.pos) else {
                    return Err(self.error("sql.unterminated-literal"));
                };
                self.bump();
                let decoded = match escaped {
                    b'0' => 0,
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    b'b' => 8,
                    b'Z' => 26,
                    // MySQL preserves the backslash for the LIKE metacharacters.
                    b'%' | b'_' => {
                        value.push(b'\\');
                        escaped
                    }
                    other => other,
                };
                value.push(decoded);
            } else {
                value.push(c);
            }
        }
        Err(self.error("sql.unterminated-literal"))
    }
}

fn identifier(tokens: &[Token], index: usize) -> Result<String, DumpError> {
    tokens
        .get(index)
        .ok_or(DumpError {
            line: tokens.first().map_or(1, |t| t.line),
            code: "sql.syntax",
        })?
        .identifier()
}
fn charset(tokens: &[Token]) -> Option<String> {
    for (i, t) in tokens.iter().enumerate() {
        if t.word("CHARSET") {
            let at = i
                + 1
                + usize::from(
                    tokens.get(i + 1).is_some_and(|t| t.symbol(b'=')),
                );
            return tokens.get(at).and_then(|t| t.identifier().ok());
        }
        if t.word("CHARACTER")
            && tokens.get(i + 1).is_some_and(|t| t.word("SET"))
        {
            return tokens.get(i + 2).and_then(|t| t.identifier().ok());
        }
    }
    None
}
fn parenthesized(
    tokens: &[Token],
    start: usize,
) -> Result<(usize, Vec<&[Token]>), DumpError> {
    let mut depth = 0usize;
    let mut begin = start + 1;
    let mut groups = Vec::new();
    for i in start..tokens.len() {
        if tokens[i].symbol(b'(') {
            depth += 1;
        }
        if tokens[i].symbol(b')') {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| tokens[i].error("sql.syntax"))?;
            if depth == 0 {
                groups.push(&tokens[begin..i]);
                return Ok((i, groups));
            }
        }
        if depth == 1 && tokens[i].symbol(b',') {
            groups.push(&tokens[begin..i]);
            begin = i + 1;
        }
    }
    Err(tokens[start].error("sql.syntax"))
}

/// Read the bounded data-only subset produced by mysqldump, never connecting to a database.
pub fn read_mysql_dump(bytes: &[u8]) -> Result<SqlDump, DumpError> {
    if bytes.len() > MAX_BYTES {
        return Err(DumpError {
            line: 1,
            code: "sql.limit",
        });
    }
    let tokens = Lexer {
        bytes,
        pos: 0,
        line: 1,
        conditional: false,
    }
    .tokens()?;
    let mut dump = SqlDump::default();
    let mut declared = BTreeSet::new();
    let mut row_count = 0;
    for statement in tokens.split(|t| t.symbol(b';')).filter(|s| !s.is_empty())
    {
        let first = &statement[0];
        // A view is metadata only. Reject SQL with side effects even in an executable comment.
        if statement.iter().any(|t| {
            [
                "OUTFILE",
                "DUMPFILE",
                "INTO",
                "LOAD_FILE",
                "SLEEP",
                "BENCHMARK",
            ]
            .iter()
            .any(|s| t.word(s))
        }) && !first.word("INSERT")
        {
            return Err(first.error("sql.unsupported-statement"));
        }
        if first.word("CREATE")
            && statement.get(1).is_some_and(|t| t.word("TABLE"))
        {
            let name = identifier(statement, 2)?;
            if !declared.insert(name.clone()) {
                return Err(first.error("sql.duplicate-table"));
            }
            if !statement.get(3).is_some_and(|t| t.symbol(b'(')) {
                return Err(first.error("sql.syntax"));
            }
            let (end, groups) = parenthesized(statement, 3)?;
            let mut table = SqlTable {
                charset: charset(&statement[end + 1..]),
                ..SqlTable::default()
            };
            for group in groups {
                let Some(head) = group.first() else {
                    return Err(first.error("sql.syntax"));
                };
                if matches!(head.kind, Kind::Ident(_))
                    || (!head.word("PRIMARY")
                        && !head.word("UNIQUE")
                        && !head.word("KEY")
                        && !head.word("CONSTRAINT")
                        && !head.word("FULLTEXT")
                        && !head.word("CHECK")
                        && !head.word("FOREIGN"))
                {
                    let column = head.identifier()?;
                    if table.columns.contains(&column) {
                        return Err(head.error("sql.duplicate-column"));
                    }
                    if let Some(cs) = charset(group) {
                        table.column_charsets.insert(column.clone(), cs);
                    }
                    table.columns.push(column);
                } else if head.word("PRIMARY") || head.word("UNIQUE") {
                    let at = group
                        .iter()
                        .position(|t| t.symbol(b'('))
                        .ok_or_else(|| head.error("sql.syntax"))?;
                    let (_, keys) = parenthesized(group, at)?;
                    let key: Vec<String> = keys
                        .iter()
                        .map(|k| identifier(k, 0))
                        .collect::<Result<_, _>>()?;
                    if head.word("PRIMARY") {
                        table.primary_key = key.clone();
                    }
                    table.unique_keys.push(key);
                }
            }
            if table.columns.is_empty() {
                return Err(first.error("sql.empty-schema"));
            }
            dump.tables.insert(name, table);
        } else if first.word("INSERT") {
            if !statement.get(1).is_some_and(|t| t.word("INTO")) {
                return Err(first.error("sql.unsupported-statement"));
            }
            let name = identifier(statement, 2)?;
            let table = dump
                .tables
                .get_mut(&name)
                .ok_or_else(|| first.error("sql.undeclared-table"))?;
            let mut at = 3;
            let order: Vec<usize> = if statement
                .get(at)
                .is_some_and(|t| t.symbol(b'('))
            {
                let (end, columns) = parenthesized(statement, at)?;
                at = end + 1;
                let mut seen = BTreeSet::new();
                let order = columns
                    .iter()
                    .map(|g| {
                        let column = identifier(g, 0)?;
                        let index = table
                            .columns
                            .iter()
                            .position(|c| *c == column)
                            .ok_or_else(|| first.error("sql.unknown-column"))?;
                        if !seen.insert(index) {
                            return Err(first.error("sql.duplicate-column"));
                        }
                        Ok(index)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if order.len() != table.columns.len() {
                    return Err(first.error("sql.row-width"));
                }
                order
            } else {
                (0..table.columns.len()).collect()
            };
            if !statement.get(at).is_some_and(|t| t.word("VALUES")) {
                return Err(first.error("sql.unsupported-statement"));
            }
            at += 1;
            if at == statement.len() {
                return Err(first.error("sql.syntax"));
            }
            while at < statement.len() {
                if !statement[at].symbol(b'(') {
                    return Err(statement[at].error("sql.syntax"));
                }
                let (end, values) = parenthesized(statement, at)?;
                if values.len() != order.len() {
                    return Err(statement[at].error("sql.row-width"));
                }
                let mut row = vec![SqlValue::Null; order.len()];
                for (i, value) in values.iter().enumerate() {
                    row[order[i]] = literal(value)?;
                }
                table.rows.push(row);
                row_count += 1;
                if row_count > MAX_ROWS {
                    return Err(first.error("sql.limit"));
                }
                at = end + 1;
                if at < statement.len() {
                    if !statement[at].symbol(b',') {
                        return Err(statement[at].error("sql.syntax"));
                    }
                    at += 1;
                    if at == statement.len() {
                        return Err(first.error("sql.syntax"));
                    }
                }
            }
        } else if first.word("DROP") {
            if statement.len() != 5
                || !statement[2].word("IF")
                || !statement[3].word("EXISTS")
                || !(statement[1].word("TABLE") || statement[1].word("VIEW"))
            {
                return Err(first.error("sql.unsupported-statement"));
            }
            let name = identifier(statement, 4)?;
            if dump.tables.get(&name).is_some_and(|t| !t.rows.is_empty()) {
                return Err(first.error("sql.destructive-statement"));
            }
            dump.tables.remove(&name);
            if statement[1].word("VIEW") {
                dump.views.remove(&name);
            }
        } else if first.word("SET") {
            if !set_envelope(statement) {
                return Err(first.error("sql.unsupported-statement"));
            }
            if statement.iter().any(|t| matches!(&t.kind,Kind::Literal(b) if b.windows(20).any(|w|w.eq_ignore_ascii_case(b"NO_BACKSLASH_ESCAPES")))) {
                return Err(first.error("sql.unsupported-mode"));
            }
            // Scalar assignments in the dump envelope, never expressions or calls.
            if statement
                .iter()
                .any(|t| t.symbol(b'(') || t.word("SELECT") || t.word("GLOBAL"))
            {
                return Err(first.error("sql.unsupported-statement"));
            }
            if statement.len() < 3 {
                return Err(first.error("sql.syntax"));
            }
        } else if first.word("LOCK") {
            if statement.len() != 4
                || !statement[1].word("TABLES")
                || !statement[3].word("WRITE")
                || !dump.tables.contains_key(&identifier(statement, 2)?)
            {
                return Err(first.error("sql.unsupported-statement"));
            }
        } else if first.word("UNLOCK") {
            if statement.len() != 2 || !statement[1].word("TABLES") {
                return Err(first.error("sql.unsupported-statement"));
            }
        } else if first.word("ALTER") {
            if statement.len() != 5
                || !statement[1].word("TABLE")
                || !(statement[3].word("ENABLE")
                    || statement[3].word("DISABLE"))
                || !statement[4].word("KEYS")
                || !dump.tables.contains_key(&identifier(statement, 2)?)
            {
                return Err(first.error("sql.unsupported-statement"));
            }
        } else if first.word("CREATE")
            && statement.iter().any(|t| t.word("VIEW"))
        {
            let at = statement
                .iter()
                .position(|t| t.word("VIEW"))
                .ok_or_else(|| first.error("sql.syntax"))?;
            let name = identifier(statement, at + 1)?;
            if !statement.get(at + 2).is_some_and(|t| t.word("AS"))
                || !statement.get(at + 3).is_some_and(|t| t.word("SELECT"))
            {
                return Err(first.error("sql.unsupported-statement"));
            }
            if !dump.views.insert(name) {
                return Err(first.error("sql.duplicate-view"));
            }
        } else {
            return Err(first.error("sql.unsupported-statement"));
        }
    }
    Ok(dump)
}

fn literal(tokens: &[Token]) -> Result<SqlValue, DumpError> {
    let first = tokens.first().ok_or(DumpError {
        line: 1,
        code: "sql.value",
    })?;
    if tokens.len() == 1 {
        match &first.kind {
            Kind::Literal(b) => return Ok(SqlValue::Bytes(b.clone())),
            Kind::Word(_) if first.word("NULL") => return Ok(SqlValue::Null),
            Kind::Word(b)
                if b.iter().all(|c| {
                    c.is_ascii_digit()
                        || matches!(c, b'-' | b'.' | b'e' | b'E' | b'+')
                }) && b.iter().any(u8::is_ascii_digit) =>
            {
                return Ok(SqlValue::Bytes(b.clone()))
            }
            _ => {}
        }
    }
    if tokens.len() == 2 {
        if first.word("_binary") {
            if let Kind::Literal(b) = &tokens[1].kind {
                return Ok(SqlValue::Bytes(b.clone()));
            }
        }
        if first.word("x") {
            if let Kind::Literal(b) = &tokens[1].kind {
                if b.len() % 2 != 0 {
                    return Err(first.error("sql.value"));
                }
                let mut value = Vec::with_capacity(b.len() / 2);
                for pair in b.chunks_exact(2) {
                    let digit = |b: u8| {
                        (b as char)
                            .to_digit(16)
                            .ok_or_else(|| first.error("sql.value"))
                    };
                    value.push((digit(pair[0])? * 16 + digit(pair[1])?) as u8);
                }
                return Ok(SqlValue::Bytes(value));
            }
        }
    }
    Err(first.error("sql.value"))
}

fn set_envelope(tokens: &[Token]) -> bool {
    const KEYS: &[&str] = &[
        "CHARACTER_SET_CLIENT",
        "CHARACTER_SET_RESULTS",
        "COLLATION_CONNECTION",
        "FOREIGN_KEY_CHECKS",
        "SQL_MODE",
        "SQL_NOTES",
        "TIME_ZONE",
        "UNIQUE_CHECKS",
        "OLD_CHARACTER_SET_CLIENT",
        "OLD_CHARACTER_SET_RESULTS",
        "OLD_COLLATION_CONNECTION",
        "OLD_FOREIGN_KEY_CHECKS",
        "OLD_SQL_MODE",
        "OLD_SQL_NOTES",
        "OLD_TIME_ZONE",
        "OLD_UNIQUE_CHECKS",
        "saved_col_connection",
        "saved_cs_client",
        "saved_cs_results",
    ];
    if tokens.get(1).is_some_and(|t| t.word("NAMES")) {
        return tokens.len() == 3
            && tokens[2].identifier().is_ok_and(|s| {
                ["utf8", "utf8mb3", "utf8mb4", "latin1"].contains(&s.as_str())
            });
    }
    let mut target = true;
    for token in &tokens[1..] {
        if token.symbol(b',') {
            target = true;
            continue;
        }
        if target {
            if token.symbol(b'@') {
                continue;
            }
            if !KEYS.iter().any(|k| token.word(k)) {
                return false;
            }
            target = false;
        }
    }
    !target && tokens.iter().any(|t| t.symbol(b'='))
}
