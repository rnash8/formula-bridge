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
// Excel and ODF (ODFF) formula syntax. Anything not on this list is rejected
// in strict mode rather than guessed at. Left out on purpose because the two
// sides disagree in some detail: TEXT and FORMAT-style functions (format
// codes are locale dependent), CEILING and FLOOR (different handling of
// negative numbers and the optional mode argument), and the newer dynamic
// array functions, which have no ODF counterpart in older releases.
const KNOWN_FUNCTIONS: &[&str] = &[
    // aggregation and conditional aggregation
    "SUM", "AVERAGE", "COUNT", "COUNTA", "COUNTBLANK", "MAX", "MIN", "MAXA", "MINA",
    "PRODUCT", "SUMPRODUCT", "SUMIF", "SUMIFS", "COUNTIF", "COUNTIFS",
    "AVERAGEIF", "AVERAGEIFS", "MEDIAN", "LARGE", "SMALL", "STDEV", "VAR",
    // logic
    "IF", "AND", "OR", "NOT", "XOR", "IFERROR", "IFNA", "TRUE", "FALSE", "CHOOSE",
    // math
    "ROUND", "ROUNDUP", "ROUNDDOWN", "ABS", "SQRT", "POWER", "MOD", "INT", "TRUNC",
    "SIGN", "EXP", "LN", "LOG10", "FACT", "EVEN", "ODD", "GCD", "LCM", "PI",
    "SIN", "COS", "TAN", "ASIN", "ACOS", "ATAN", "ATAN2", "RAND", "RANDBETWEEN",
    // text
    "CONCATENATE", "LEN", "LEFT", "RIGHT", "MID", "TRIM", "UPPER", "LOWER", "PROPER",
    "FIND", "SEARCH", "SUBSTITUTE", "REPLACE", "REPT", "EXACT", "VALUE", "CHAR",
    "CODE", "CLEAN",
    // lookup and reference
    "VLOOKUP", "HLOOKUP", "LOOKUP", "INDEX", "MATCH", "OFFSET", "ROW", "COLUMN",
    "ROWS", "COLUMNS",
    // information
    "ISNUMBER", "ISTEXT", "ISBLANK", "ISERROR", "ISERR", "ISNA", "ISLOGICAL", "NA",
    // date and time
    "TODAY", "NOW", "DATE", "TIME", "YEAR", "MONTH", "DAY", "HOUR", "MINUTE",
    "SECOND", "WEEKDAY", "DATEVALUE", "EDATE", "EOMONTH",
];

pub fn is_known_function(name_upper: &str) -> bool {
    KNOWN_FUNCTIONS.contains(&name_upper)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_has_no_duplicates() {
        let mut names: Vec<&str> = KNOWN_FUNCTIONS.to_vec();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }

    #[test]
    fn allowlist_is_uppercase() {
        for name in KNOWN_FUNCTIONS {
            assert_eq!(*name, name.to_uppercase());
        }
    }

    #[test]
    fn lookup_matches_only_listed_names() {
        assert!(is_known_function("SUMIFS"));
        assert!(is_known_function("EOMONTH"));
        assert!(!is_known_function("XLOOKUP"));
        assert!(!is_known_function("sum"));
    }
}
