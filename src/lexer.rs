use crate::dialect::Dialect;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(String),
    Str(String),
    Ident(String),
    Ref(CellRef),
    ArgSep,
    LParen,
    RParen,
    Op(String),
    // Only produced in lenient mode, for characters the lexer could not
    // classify. Passed through to the output verbatim.
    Raw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CellAddr {
    pub col_abs: bool,
    pub col: String,
    pub row_abs: bool,
    pub row: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CellRef {
    pub sheet: Option<String>,
    pub start: CellAddr,
    pub end: Option<CellAddr>,
}

#[derive(Debug)]
pub struct LexError {
    pub pos: usize,
    pub message: String,
}

struct Cursor {
    chars: Vec<char>,
    pos: usize,
}

impl Cursor {
    fn new(input: &str) -> Self {
        Cursor { chars: input.chars().collect(), pos: 0 }
    }
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn peek_at(&self, off: usize) -> Option<char> {
        self.chars.get(self.pos + off).copied()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }
}

pub fn lex(input: &str, dialect: Dialect, lenient: bool) -> Result<Vec<Token>, LexError> {
    let mut cur = Cursor::new(input);
    let mut tokens = Vec::new();
    while let Some(c) = cur.peek() {
        if c.is_whitespace() {
            cur.bump();
            continue;
        }
        let tok = match c {
            '(' => { cur.bump(); Token::LParen }
            ')' => { cur.bump(); Token::RParen }
            ',' if dialect == Dialect::Excel => { cur.bump(); Token::ArgSep }
            ';' if dialect == Dialect::Odf => { cur.bump(); Token::ArgSep }
            '"' => lex_string(&mut cur)?,
            '0'..='9' => lex_number(&mut cur),
            '[' if dialect == Dialect::Odf => lex_odf_ref(&mut cur)?,
            '\'' if dialect == Dialect::Excel => lex_excel_sheet_ref(&mut cur)?,
            'A'..='Z' | 'a'..='z' => {
                if dialect == Dialect::Excel {
                    lex_excel_word(&mut cur)?
                } else {
                    Token::Ident(collect_word(&mut cur))
                }
            }
            '$' if dialect == Dialect::Excel => lex_excel_word(&mut cur)?,
            '<' | '>' | '=' => lex_compare_op(&mut cur),
            '+' | '-' | '*' | '/' | '^' | '&' | '%' => { cur.bump(); Token::Op(c.to_string()) }
            _ => {
                if lenient {
                    cur.bump();
                    Token::Raw(c.to_string())
                } else {
                    return Err(LexError { pos: cur.pos, message: format!("unexpected character '{}'", c) });
                }
            }
        };
        tokens.push(tok);
    }
    Ok(tokens)
}

fn lex_number(cur: &mut Cursor) -> Token {
    let mut s = String::new();
    while let Some(c) = cur.peek() {
        if c.is_ascii_digit() {
            s.push(c);
            cur.bump();
        } else {
            break;
        }
    }
    if cur.peek() == Some('.') && cur.peek_at(1).map_or(false, |c| c.is_ascii_digit()) {
        s.push('.');
        cur.bump();
        while let Some(c) = cur.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                cur.bump();
            } else {
                break;
            }
        }
    }
    if matches!(cur.peek(), Some('e') | Some('E')) {
        let save_s = s.clone();
        let save_pos = cur.pos;
        let mut exp = String::new();
        exp.push(cur.bump().unwrap());
        if matches!(cur.peek(), Some('+') | Some('-')) {
            exp.push(cur.bump().unwrap());
        }
        let mut had_digit = false;
        while let Some(c) = cur.peek() {
            if c.is_ascii_digit() {
                exp.push(c);
                cur.bump();
                had_digit = true;
            } else {
                break;
            }
        }
        if had_digit {
            s.push_str(&exp);
        } else {
            cur.pos = save_pos;
            s = save_s;
        }
    }
    Token::Number(s)
}

fn lex_string(cur: &mut Cursor) -> Result<Token, LexError> {
    let start = cur.pos;
    cur.bump(); // opening quote
    let mut s = String::new();
    loop {
        match cur.peek() {
            None => return Err(LexError { pos: start, message: "unterminated string literal".to_string() }),
            Some('"') => {
                cur.bump();
                if cur.peek() == Some('"') {
                    s.push('"');
                    s.push('"');
                    cur.bump();
                } else {
                    break;
                }
            }
            Some(c) => {
                s.push(c);
                cur.bump();
            }
        }
    }
    Ok(Token::Str(s))
}

fn lex_compare_op(cur: &mut Cursor) -> Token {
    let c = cur.bump().unwrap();
    let op = match (c, cur.peek()) {
        ('<', Some('=')) => { cur.bump(); "<=".to_string() }
        ('>', Some('=')) => { cur.bump(); ">=".to_string() }
        ('<', Some('>')) => { cur.bump(); "<>".to_string() }
        _ => c.to_string(),
    };
    Token::Op(op)
}

fn collect_word(cur: &mut Cursor) -> String {
    let mut s = String::new();
    while let Some(c) = cur.peek() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '$' {
            s.push(c);
            cur.bump();
        } else {
            break;
        }
    }
    s
}

// Excel bare word: could turn out to be a sheet-qualified reference
// (WORD!ADDR), a function name (WORD followed by '('), a plain cell
// reference, or a defined name / boolean literal.
fn lex_excel_word(cur: &mut Cursor) -> Result<Token, LexError> {
    let start = cur.pos;
    let raw = collect_word(cur);
    if raw.is_empty() {
        return Err(LexError { pos: start, message: "empty token".to_string() });
    }
    if cur.peek() == Some('!') {
        cur.bump();
        return finish_excel_ref(cur, start, Some(raw));
    }
    if cur.peek() == Some('(') {
        return Ok(Token::Ident(raw));
    }
    if let Some(addr1) = parse_cell_address(&raw) {
        let mut end = None;
        if cur.peek() == Some(':') {
            cur.bump();
            let addr2_word = collect_word(cur);
            let addr2 = parse_cell_address(&addr2_word)
                .ok_or_else(|| LexError { pos: start, message: "invalid range end".to_string() })?;
            end = Some(addr2);
        }
        return Ok(Token::Ref(CellRef { sheet: None, start: addr1, end }));
    }
    Ok(Token::Ident(raw))
}

fn lex_excel_sheet_ref(cur: &mut Cursor) -> Result<Token, LexError> {
    let start = cur.pos;
    cur.bump(); // opening quote
    let mut name = String::new();
    loop {
        match cur.peek() {
            None => return Err(LexError { pos: start, message: "unterminated sheet name".to_string() }),
            Some('\'') => {
                cur.bump();
                if cur.peek() == Some('\'') {
                    name.push('\'');
                    cur.bump();
                } else {
                    break;
                }
            }
            Some(c) => {
                name.push(c);
                cur.bump();
            }
        }
    }
    if cur.peek() != Some('!') {
        return Err(LexError { pos: start, message: "expected '!' after quoted sheet name".to_string() });
    }
    cur.bump();
    finish_excel_ref(cur, start, Some(name))
}

fn finish_excel_ref(cur: &mut Cursor, start: usize, sheet: Option<String>) -> Result<Token, LexError> {
    let addr1_word = collect_word(cur);
    let addr1 = parse_cell_address(&addr1_word)
        .ok_or_else(|| LexError { pos: start, message: "invalid cell reference after sheet name".to_string() })?;
    let mut end = None;
    if cur.peek() == Some(':') {
        cur.bump();
        let addr2_word = collect_word(cur);
        let addr2 = parse_cell_address(&addr2_word)
            .ok_or_else(|| LexError { pos: start, message: "invalid range end".to_string() })?;
        end = Some(addr2);
    }
    Ok(Token::Ref(CellRef { sheet, start: addr1, end }))
}

fn parse_cell_address(word: &str) -> Option<CellAddr> {
    let chars: Vec<char> = word.chars().collect();
    let mut i = 0;
    let col_abs = if chars.get(i) == Some(&'$') { i += 1; true } else { false };
    let col_start = i;
    while i < chars.len() && chars[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i == col_start {
        return None;
    }
    let col: String = chars[col_start..i].iter().collect::<String>().to_uppercase();
    let row_abs = if chars.get(i) == Some(&'$') { i += 1; true } else { false };
    let row_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if row_start == i || i != chars.len() {
        return None;
    }
    let row: u32 = chars[row_start..i].iter().collect::<String>().parse().ok()?;
    Some(CellAddr { col_abs, col, row_abs, row })
}

// ODF references are always bracketed: [Sheet1.A1], [.A1:.B2], etc. A
// leading '$' on the sheet name marks an absolute sheet reference, which
// we fold into a plain reference since Excel has no equivalent.
fn lex_odf_ref(cur: &mut Cursor) -> Result<Token, LexError> {
    let start = cur.pos;
    cur.bump(); // consume '['
    let sheet = parse_odf_sheet_name(cur, start)?;
    if cur.peek() != Some('.') {
        return Err(LexError { pos: cur.pos, message: "expected '.' in cell reference".to_string() });
    }
    cur.bump();
    let addr1 = parse_odf_address(cur)?;
    let mut end = None;
    if cur.peek() == Some(':') {
        cur.bump();
        let _sheet2 = parse_odf_sheet_name(cur, start)?;
        if cur.peek() != Some('.') {
            return Err(LexError { pos: cur.pos, message: "expected '.' in range end".to_string() });
        }
        cur.bump();
        let addr2 = parse_odf_address(cur)?;
        end = Some(addr2);
    }
    if cur.peek() != Some(']') {
        return Err(LexError { pos: cur.pos, message: "expected ']' to close cell reference".to_string() });
    }
    cur.bump();
    Ok(Token::Ref(CellRef { sheet, start: addr1, end }))
}

fn parse_odf_sheet_name(cur: &mut Cursor, start: usize) -> Result<Option<String>, LexError> {
    match cur.peek() {
        Some('.') => Ok(None),
        Some('\'') => {
            cur.bump();
            let mut name = String::new();
            loop {
                match cur.peek() {
                    None => return Err(LexError { pos: start, message: "unterminated sheet name".to_string() }),
                    Some('\'') => {
                        cur.bump();
                        if cur.peek() == Some('\'') {
                            name.push('\'');
                            cur.bump();
                        } else {
                            break;
                        }
                    }
                    Some(c) => {
                        name.push(c);
                        cur.bump();
                    }
                }
            }
            Ok(Some(name))
        }
        _ => {
            let mut name = String::new();
            while let Some(c) = cur.peek() {
                if c == '.' || c == ':' || c == ']' {
                    break;
                }
                name.push(c);
                cur.bump();
            }
            if name.is_empty() {
                Err(LexError { pos: start, message: "expected sheet name or '.' in cell reference".to_string() })
            } else {
                Ok(Some(name.trim_start_matches('$').to_string()))
            }
        }
    }
}

fn parse_odf_address(cur: &mut Cursor) -> Result<CellAddr, LexError> {
    let col_abs = if cur.peek() == Some('$') { cur.bump(); true } else { false };
    let col_start_pos = cur.pos;
    let mut col = String::new();
    while let Some(c) = cur.peek() {
        if c.is_ascii_alphabetic() {
            col.push(c.to_ascii_uppercase());
            cur.bump();
        } else {
            break;
        }
    }
    if col.is_empty() {
        return Err(LexError { pos: col_start_pos, message: "expected column letters in cell reference".to_string() });
    }
    let row_abs = if cur.peek() == Some('$') { cur.bump(); true } else { false };
    let row_start_pos = cur.pos;
    let mut row_s = String::new();
    while let Some(c) = cur.peek() {
        if c.is_ascii_digit() {
            row_s.push(c);
            cur.bump();
        } else {
            break;
        }
    }
    if row_s.is_empty() {
        return Err(LexError { pos: row_start_pos, message: "expected row number in cell reference".to_string() });
    }
    let row: u32 = row_s
        .parse()
        .map_err(|_| LexError { pos: row_start_pos, message: "row number out of range".to_string() })?;
    Ok(CellAddr { col_abs, col, row_abs, row })
}
