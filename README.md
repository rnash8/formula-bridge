# formula-bridge

Excel and ODF (the format LibreOffice Calc and OpenOffice use) write
formulas that look almost identical but aren't. `=SUM(Sheet1!A1:B10,10)`
in Excel is `=SUM([Sheet1.A1:.B10];10)` in ODF: different argument
separator, different cell reference syntax, and a function set that only
partly overlaps. Copy a formula from one into the other and it either
fails to parse or, worse, parses into something else.

formula-bridge is a command-line converter between the two formula
dialects. It reads formulas one per line and writes the converted form.

## Usage

```
$ echo '=SUM(Sheet1!A1:B10,10)*IF(C1>0,1,-1)' | \
    cargo run -- --from excel --to odf
=SUM([Sheet1.A1:.B10];10)*IF([.C1]>0;1;-1)

$ echo '=SUM([Sheet1.A1:.B10];10)*IF([.C1]>0;1;-1)' | \
    cargo run -- --from odf --to excel
=SUM(Sheet1!A1:B10,10)*IF(C1>0,1,-1)
```

A file of formulas (one per line, with or without the leading `=`) works
the same way:

```
$ cargo run -- --from excel --to odf formulas.txt > converted.txt
```

## Strict by default

Only a function whose name and behavior are confirmed identical in both
dialects is converted; anything else is rejected with a line number and
column, and the run exits non-zero. This is deliberate: a silently wrong
formula in a spreadsheet is worse than a converter that refuses to guess.

```
$ echo '=XLOOKUP(A1,B:B,C:C)' | cargo run -- --from excel --to odf
line 1: unknown function 'XLOOKUP' (rerun with --lenient to pass it through unchanged)
```

Pass `--lenient` to convert everything else in the formula (references,
separators, literals) and pass unrecognized function names through
unchanged. The output may not be valid in the target application if the
function genuinely doesn't exist there - `--lenient` is an escape hatch
for names you know are fine (custom add-ins, functions missing from the
allowlist), not a correctness guarantee.

```
$ echo '=XLOOKUP(A1,B:B,C:C)' | cargo run -- --from excel --to odf --lenient
=XLOOKUP([.A1];[.B:B];[.C:C])
```

## Current limitations

- The function allowlist covers roughly three dozen common functions
  (see `KNOWN_FUNCTIONS` in `src/dialect.rs`); anything else needs
  `--lenient`.
- No array formulas, no structured table references (`Table1[Column]`),
  no 3-D references spanning multiple sheets.
- ODF absolute sheet references (`$Sheet1`) are read but downgraded to a
  plain sheet reference on output, since Excel has no equivalent.

This is an early skeleton, not a finished tool - see the issue tracker
for what's planned next.

## Building

```
cargo build --release
```

No third-party dependencies; the standard library is enough for both the
formula lexer and the CLI argument parsing.
