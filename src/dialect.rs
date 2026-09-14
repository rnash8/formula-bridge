#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Excel,
    Odf,
}

impl Dialect {
    pub fn parse(s: &str) -> Result<Dialect, String> {
        match s.to_lowercase().as_str() {
            "excel" | "xlsx" | "xls" => Ok(Dialect::Excel),
            "odf" | "ods" | "calc" => Ok(Dialect::Odf),
            other => Err(format!("unknown format '{}', expected 'excel' or 'odf'", other)),
        }
    }
}

// Functions whose name, argument order, and semantics are the same in both
// Excel and ODF (ODFF) formula syntax. Deliberately small for now: anything
// not on this list is rejected in strict mode rather than guessed at.
const KNOWN_FUNCTIONS: &[&str] = &[
    "SUM", "AVERAGE", "COUNT", "COUNTA", "MAX", "MIN", "IF", "AND", "OR", "NOT",
    "ROUND", "ROUNDUP", "ROUNDDOWN", "ABS", "SQRT", "POWER", "MOD", "INT",
    "CONCATENATE", "LEN", "LEFT", "RIGHT", "MID", "TRIM", "UPPER", "LOWER",
    "VLOOKUP", "HLOOKUP", "INDEX", "MATCH", "IFERROR", "ISNUMBER", "ISTEXT",
    "ISBLANK", "TODAY", "NOW", "DATE", "YEAR", "MONTH", "DAY", "TRUE", "FALSE",
];

pub fn is_known_function(name_upper: &str) -> bool {
    KNOWN_FUNCTIONS.contains(&name_upper)
}
