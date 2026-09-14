use crate::dialect::{self, Dialect};
use crate::lexer::{self, CellAddr, CellRef, Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Strict,
    Lenient,
}

#[derive(Debug)]
pub struct ConvertError {
    pub message: String,
}

pub fn convert(input: &str, from: Dialect, to: Dialect, mode: Mode) -> Result<String, ConvertError> {
    let lenient = mode == Mode::Lenient;
    let tokens = lexer::lex(input, from, lenient)
        .map_err(|e| ConvertError { message: format!("column {}: {}", e.pos + 1, e.message) })?;
    render(&tokens, to, mode)
}

fn render(tokens: &[Token], to: Dialect, mode: Mode) -> Result<String, ConvertError> {
    let mut out = String::new();
    for (i, tok) in tokens.iter().enumerate() {
        match tok {
            // Numbers and strings use the same literal syntax in both
            // dialects, so they pass through untouched.
            Token::Number(s) => out.push_str(s),
            Token::Str(s) => {
                out.push('"');
                out.push_str(s);
                out.push('"');
            }
            Token::Ident(name) => {
                let is_call = matches!(tokens.get(i + 1), Some(Token::LParen));
                if is_call {
                    let upper = name.to_uppercase();
                    if dialect::is_known_function(&upper) {
                        out.push_str(&upper);
                    } else if mode == Mode::Lenient {
                        out.push_str(name);
                    } else {
                        return Err(ConvertError {
                            message: format!(
                                "unknown function '{}' (rerun with --lenient to pass it through unchanged)",
                                name
                            ),
                        });
                    }
                } else {
                    out.push_str(name);
                }
            }
            Token::Ref(r) => out.push_str(&render_ref(r, to)),
            Token::ArgSep => out.push(match to {
                Dialect::Excel => ',',
                Dialect::Odf => ';',
            }),
            Token::LParen => out.push('('),
            Token::RParen => out.push(')'),
            Token::Op(s) => out.push_str(s),
            Token::Raw(s) => out.push_str(s),
        }
    }
    Ok(out)
}

fn render_ref(r: &CellRef, to: Dialect) -> String {
    match to {
        Dialect::Excel => {
            let mut s = String::new();
            if let Some(sheet) = &r.sheet {
                if sheet_needs_quoting(sheet) {
                    s.push('\'');
                    s.push_str(&sheet.replace('\'', "''"));
                    s.push('\'');
                } else {
                    s.push_str(sheet);
                }
                s.push('!');
            }
            s.push_str(&render_addr(&r.start));
            if let Some(end) = &r.end {
                s.push(':');
                s.push_str(&render_addr(end));
            }
            s
        }
        Dialect::Odf => {
            let mut s = String::from("[");
            if let Some(sheet) = &r.sheet {
                s.push_str(sheet);
            }
            s.push('.');
            s.push_str(&render_addr(&r.start));
            if let Some(end) = &r.end {
                s.push_str(":.");
                s.push_str(&render_addr(end));
            }
            s.push(']');
            s
        }
    }
}

fn render_addr(addr: &CellAddr) -> String {
    let mut s = String::new();
    if addr.col_abs {
        s.push('$');
    }
    s.push_str(&addr.col);
    if addr.row_abs {
        s.push('$');
    }
    s.push_str(&addr.row.to_string());
    s
}

fn sheet_needs_quoting(name: &str) -> bool {
    name.is_empty()
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        || name.chars().next().map_or(false, |c| c.is_ascii_digit())
}
