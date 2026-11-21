//! A small TOML subset parser (no dependencies).
//!
//! Supports what motd-forge configs and themes use: comments, `key = value`,
//! `[table]` and `[[array.of.tables]]` headers, dotted keys, basic/literal strings
//! (single and multi-line), integers, floats, booleans, arrays and inline tables.
//! Not supported: dates/times, hex/octal/binary integers, special floats.
use std::collections::BTreeMap;

pub type Table = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Arr(Vec<Value>),
    Table(Table),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        if let Value::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
    pub fn as_arr(&self) -> Option<&[Value]> {
        if let Value::Arr(a) = self {
            Some(a)
        } else {
            None
        }
    }
    pub fn as_table(&self) -> Option<&Table> {
        if let Value::Table(t) = self {
            Some(t)
        } else {
            None
        }
    }
}

pub fn parse(src: &str) -> Result<Table, String> {
    let mut p = Parser {
        c: src.chars().collect(),
        i: 0,
        line: 1,
    };
    p.document()
}

struct Parser {
    c: Vec<char>,
    i: usize,
    line: usize,
}

type R<T> = Result<T, String>;

impl Parser {
    fn err<T>(&self, msg: &str) -> R<T> {
        Err(format!("line {}: {}", self.line, msg))
    }
    fn peek(&self) -> Option<char> {
        self.c.get(self.i).copied()
    }
    fn starts(&self, s: &str) -> bool {
        s.chars()
            .enumerate()
            .all(|(k, ch)| self.c.get(self.i + k) == Some(&ch))
    }
    fn bump(&mut self) -> Option<char> {
        let ch = self.c.get(self.i).copied();
        if ch == Some('\n') {
            self.line += 1;
        }
        if ch.is_some() {
            self.i += 1;
        }
        ch
    }
    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.i += 1;
        }
    }
    fn skip_comment(&mut self) {
        if self.peek() == Some('#') {
            while !matches!(self.peek(), Some('\n') | None) {
                self.i += 1;
            }
        }
    }
    /// Whitespace, newlines and comments.
    fn skip_all(&mut self) {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\r' | '\n') => {
                    self.bump();
                }
                Some('#') => self.skip_comment(),
                _ => break,
            }
        }
    }
    fn end_of_line(&mut self) -> R<()> {
        self.skip_spaces();
        self.skip_comment();
        match self.peek() {
            None => Ok(()),
            Some('\r') => {
                self.i += 1;
                self.end_of_line()
            }
            Some('\n') => {
                self.bump();
                Ok(())
            }
            Some(_) => self.err("unexpected characters after value"),
        }
    }

    fn document(&mut self) -> R<Table> {
        let mut root = Table::new();
        let mut path: Vec<String> = Vec::new();
        loop {
            self.skip_all();
            match self.peek() {
                None => return Ok(root),
                Some('[') => {
                    let is_array = self.starts("[[");
                    self.i += if is_array { 2 } else { 1 };
                    let keys = self.key_path()?;
                    self.skip_spaces();
                    let close = if is_array { "]]" } else { "]" };
                    if !self.starts(close) {
                        return self.err("expected closing bracket in table header");
                    }
                    self.i += close.len();
                    if is_array {
                        let (last, parent) = keys.split_last().ok_or("empty table header")?;
                        let tbl = resolve(&mut root, parent).map_err(|e| self.fail(&e))?;
                        match tbl
                            .entry(last.clone())
                            .or_insert_with(|| Value::Arr(Vec::new()))
                        {
                            Value::Arr(a) => a.push(Value::Table(Table::new())),
                            _ => return self.err("key is not an array of tables"),
                        }
                    } else {
                        resolve(&mut root, &keys).map_err(|e| self.fail(&e))?;
                    }
                    path = keys;
                    self.end_of_line()?;
                }
                Some(_) => {
                    let keys = self.key_path()?;
                    self.skip_spaces();
                    if self.peek() != Some('=') {
                        return self.err("expected '=' after key");
                    }
                    self.i += 1;
                    self.skip_spaces();
                    let v = self.value()?;
                    let (last, parent) = keys.split_last().ok_or("empty key")?;
                    let mut full = path.clone();
                    full.extend_from_slice(parent);
                    let tbl = resolve(&mut root, &full).map_err(|e| self.fail(&e))?;
                    if tbl.insert(last.clone(), v).is_some() {
                        return self.err(&format!("duplicate key '{last}'"));
                    }
                    self.end_of_line()?;
                }
            }
        }
    }

    fn fail(&self, msg: &str) -> String {
        format!("line {}: {}", self.line, msg)
    }

    fn key_path(&mut self) -> R<Vec<String>> {
        let mut keys = Vec::new();
        loop {
            self.skip_spaces();
            keys.push(self.key()?);
            self.skip_spaces();
            if self.peek() == Some('.') {
                self.i += 1;
            } else {
                return Ok(keys);
            }
        }
    }

    fn key(&mut self) -> R<String> {
        match self.peek() {
            Some('"') => self.basic_string(),
            Some('\'') => self.literal_string(),
            _ => {
                let start = self.i;
                while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    self.i += 1;
                }
                if self.i == start {
                    return self.err("expected a key");
                }
                Ok(self.c[start..self.i].iter().collect())
            }
        }
    }

    fn value(&mut self) -> R<Value> {
        match self.peek() {
            Some('"') => {
                if self.starts("\"\"\"") {
                    self.multi_basic().map(Value::Str)
                } else {
                    self.basic_string().map(Value::Str)
                }
            }
            Some('\'') => {
                if self.starts("'''") {
                    self.multi_literal().map(Value::Str)
                } else {
                    self.literal_string().map(Value::Str)
                }
            }
            Some('[') => self.array(),
            Some('{') => self.inline_table(),
            Some('t') if self.starts("true") => {
                self.i += 4;
                Ok(Value::Bool(true))
            }
            Some('f') if self.starts("false") => {
                self.i += 5;
                Ok(Value::Bool(false))
            }
            Some(c) if c.is_ascii_digit() || c == '+' || c == '-' => self.number(),
            _ => self.err("expected a value"),
        }
    }

    fn number(&mut self) -> R<Value> {
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || "+-._eE".contains(c)) {
            self.i += 1;
        }
        let text: String = self.c[start..self.i]
            .iter()
            .filter(|c| **c != '_')
            .collect();
        if let Ok(i) = text.parse::<i64>() {
            Ok(Value::Int(i))
        } else if let Ok(f) = text.parse::<f64>() {
            Ok(Value::Float(f))
        } else {
            self.err(&format!("invalid number '{text}'"))
        }
    }

    fn array(&mut self) -> R<Value> {
        self.i += 1; // [
        let mut items = Vec::new();
        loop {
            self.skip_all();
            if self.peek() == Some(']') {
                self.i += 1;
                return Ok(Value::Arr(items));
            }
            items.push(self.value()?);
            self.skip_all();
            match self.peek() {
                Some(',') => self.i += 1,
                Some(']') => {}
                _ => return self.err("expected ',' or ']' in array"),
            }
        }
    }

    fn inline_table(&mut self) -> R<Value> {
        self.i += 1; // {
        let mut t = Table::new();
        loop {
            self.skip_spaces();
            if self.peek() == Some('}') {
                self.i += 1;
                return Ok(Value::Table(t));
            }
            let keys = self.key_path()?;
            self.skip_spaces();
            if self.peek() != Some('=') {
                return self.err("expected '=' in inline table");
            }
            self.i += 1;
            self.skip_spaces();
            let v = self.value()?;
            let (last, parent) = keys.split_last().ok_or("empty key")?;
            let tbl = resolve(&mut t, parent).map_err(|e| self.fail(&e))?;
            tbl.insert(last.clone(), v);
            self.skip_spaces();
            match self.peek() {
                Some(',') => self.i += 1,
                Some('}') => {}
                _ => return self.err("expected ',' or '}' in inline table"),
            }
        }
    }

    fn literal_string(&mut self) -> R<String> {
        self.i += 1; // '
        let start = self.i;
        while !matches!(self.peek(), Some('\'') | Some('\n') | None) {
            self.i += 1;
        }
        if self.peek() != Some('\'') {
            return self.err("unterminated literal string");
        }
        let s = self.c[start..self.i].iter().collect();
        self.i += 1;
        Ok(s)
    }

    fn multi_literal(&mut self) -> R<String> {
        self.i += 3;
        self.skip_first_newline();
        let mut s = String::new();
        loop {
            if self.starts("'''") {
                self.i += 3;
                for _ in 0..2 {
                    if s_extra(self, '\'') {
                        s.push('\'');
                    }
                }
                return Ok(s);
            }
            match self.bump() {
                Some('\r') if self.peek() == Some('\n') => {}
                Some(c) => s.push(c),
                None => return self.err("unterminated multi-line literal string"),
            }
        }
    }

    fn skip_first_newline(&mut self) {
        if self.peek() == Some('\r') && self.c.get(self.i + 1) == Some(&'\n') {
            self.i += 1;
        }
        if self.peek() == Some('\n') {
            self.bump();
        }
    }

    fn basic_string(&mut self) -> R<String> {
        self.i += 1; // "
        let mut s = String::new();
        loop {
            match self.bump() {
                Some('"') => return Ok(s),
                Some('\\') => s.push(self.escape()?),
                Some('\n') | None => return self.err("unterminated string"),
                Some(c) => s.push(c),
            }
        }
    }

    fn multi_basic(&mut self) -> R<String> {
        self.i += 3;
        self.skip_first_newline();
        let mut s = String::new();
        loop {
            if self.starts("\"\"\"") {
                self.i += 3;
                for _ in 0..2 {
                    if s_extra(self, '"') {
                        s.push('"');
                    }
                }
                return Ok(s);
            }
            match self.bump() {
                Some('\\') => {
                    // line-ending backslash trims following whitespace and newlines
                    let mut j = self.i;
                    while matches!(self.c.get(j), Some(' ' | '\t' | '\r')) {
                        j += 1;
                    }
                    if self.c.get(j) == Some(&'\n') {
                        while matches!(self.peek(), Some(' ' | '\t' | '\r' | '\n')) {
                            self.bump();
                        }
                    } else {
                        s.push(self.escape()?);
                    }
                }
                Some('\r') if self.peek() == Some('\n') => {}
                Some(c) => s.push(c),
                None => return self.err("unterminated multi-line string"),
            }
        }
    }

    fn escape(&mut self) -> R<char> {
        Ok(match self.bump() {
            Some('b') => '\u{8}',
            Some('t') => '\t',
            Some('n') => '\n',
            Some('f') => '\u{c}',
            Some('r') => '\r',
            Some('"') => '"',
            Some('\\') => '\\',
            Some(k @ ('u' | 'U')) => {
                let n = if k == 'u' { 4 } else { 8 };
                let hex: String = (0..n).filter_map(|_| self.bump()).collect();
                u32::from_str_radix(&hex, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| self.fail("invalid unicode escape"))?
            }
            _ => return self.err("invalid escape sequence"),
        })
    }
}

/// After the closing delimiter of a multi-line string, up to two extra quotes belong to the string.
fn s_extra(p: &mut Parser, q: char) -> bool {
    if p.peek() == Some(q) {
        p.i += 1;
        true
    } else {
        false
    }
}

/// Walk (creating as needed) to the table at `path`; arrays of tables resolve to their last element.
fn resolve<'a>(mut t: &'a mut Table, path: &[String]) -> Result<&'a mut Table, String> {
    for k in path {
        let v = t
            .entry(k.clone())
            .or_insert_with(|| Value::Table(Table::new()));
        t = match v {
            Value::Table(inner) => inner,
            Value::Arr(a) => match a.last_mut() {
                Some(Value::Table(inner)) => inner,
                _ => return Err(format!("'{k}' is not a table")),
            },
            _ => return Err(format!("'{k}' is not a table")),
        };
    }
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_and_tables() {
        let t = parse("a = 1\nb = 2.5\nc = true\nd = \"x\\ty\" # c\n[m]\nk = 'lit'\n").unwrap();
        assert_eq!(t["a"], Value::Int(1));
        assert_eq!(t["b"], Value::Float(2.5));
        assert_eq!(t["c"], Value::Bool(true));
        assert_eq!(t["d"].as_str(), Some("x\ty"));
        assert_eq!(t["m"].as_table().unwrap()["k"].as_str(), Some("lit"));
    }

    #[test]
    fn arrays_and_array_tables() {
        let t = parse("r = [\n [\"a\", \"b\"],\n [\"c\", \"d\"],\n]\n[[s]]\nx = 1\n[[s]]\nx = 2\n")
            .unwrap();
        assert_eq!(t["r"].as_arr().unwrap().len(), 2);
        let s = t["s"].as_arr().unwrap();
        assert_eq!(s[1].as_table().unwrap()["x"], Value::Int(2));
    }

    #[test]
    fn multiline_strings() {
        let t = parse("a = '''\nl1 \\ {x}\nl2\n'''\nb = \"\"\"\nq\\\\r\n\"\"\"\n").unwrap();
        assert_eq!(t["a"].as_str(), Some("l1 \\ {x}\nl2\n"));
        assert_eq!(t["b"].as_str(), Some("q\\r\n"));
    }

    #[test]
    fn inline_tables_and_dotted_keys() {
        let t = parse("a = { x = 1, y = \"z\" }\nb.c = 3\n").unwrap();
        assert_eq!(t["a"].as_table().unwrap()["y"].as_str(), Some("z"));
        assert_eq!(t["b"].as_table().unwrap()["c"], Value::Int(3));
    }

    #[test]
    fn errors() {
        assert!(parse("a = \n").is_err());
        assert!(parse("a = 1\na = 2\n").is_err());
        assert!(parse("a = \"x\n").is_err());
    }
}
