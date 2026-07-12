# Runestaff

Rust workspace: Java bytecode assembler/disassembler. Converts `.rns` (Rune Assembly) ↔ `.class` files. Generates arbitrary bytecode (including invalid) for JVM testing.

## Workspace

| Crate | Package name | Lib/bin name | Purpose |
|-------|-------------|-------------|---------|
| `rns-lang/` | `rns-lang` | `rns` (lib) | Core: lexer, parser, AST, assembler, disassembler |
| `rnsc/` | `rnsc` | `rnsc` (bin) | CLI: `asm`, `dis` subcommands |
| `rns-lsp/` | `rns-lsp` | `rns-lsp` (bin) | LSP server (POC only, not usable) |

**Gotcha**: crate `rns-lang` exports lib as `rns`. In Cargo.toml: `rns = { workspace = true }`.

Edition 2024. Depends on `lvm-class` (JVM class file parser/writer from Lagertha).

## Commands

```bash
# CI runs these in order — all must pass:
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace
cargo test --workspace

# Run integration tests (all .rns fixtures auto-discovered by rstest):
cargo test --test rns_test                    # all rns integration tests
cargo test --test cli_test                    # all CLI tests
cargo test -p rns-lang                        # lexer unit tests

# Run a single test by fixture name (substring match):
cargo test --test rns_test -- missing_operand
cargo test --test rns_test -- HelloWorld

# Snapshot workflow (insta):
cargo insta review   # interactive review of changed snapshots
cargo insta accept   # accept all pending snapshots
```

## External dependency: `javap`

Integration tests for successful assembly call `javap -v -p` on generated `.class` files and snapshot the output. JDK must be installed and `javap` on PATH. Tests fail without it.

## Writing integration tests

### Test directories (`rnsc/test_data/rns_integration/`)

| Directory | Purpose |
|-----------|---------|
| `error/lexer/` | Lexer error tests (unclosed quotes, invalid escapes, unknown directives) |
| `error/parser/<directive>/` | Parser errors grouped by directive |
| `error/parser/outside_class_directive/before_class_dir/` | Tokens before `.class` |
| `error/parser/outside_class_directive/after_class_dir/` | Tokens after `.class_end` |
| `general/` | Valid bytecode (must produce correct `.class`) |
| `jvms/` | JVM spec compliance warnings |
| `rns_warn/` | Assembler/parser warning tests |
| `integration_to_be_migrated/` | Legacy — don't add new tests here |

**Test directory placement rules:**

- `error/parser/` — only tests that produce at least one **syntax error** (E-XXX). Snapshot shows `not generated` for DISASSEMBLED/JAVAP.
- `rns_warn/` — tests that produce only **warnings** (W-XXX, JVMS-XXX). May assemble successfully.
- `general/` — tests that assemble **successfully** with **zero diagnostics** (no errors, no warnings).

If a test produces only warnings or assembles silently, it MUST NOT be in `error/parser/`.

### Test naming convention (per directive/feature)

| Pattern | Purpose |
|---------|---------|
| `<feature>.rns` | Primary error condition |
| `<feature>_recovers.rns` | Parser continues after error, reports subsequent errors too |
| `multiple_definitions.rns` | Directive appears twice |
| `multiple_definitions_recovers.rns` | Duplicate + subsequent error |
| `missing_operand.rns` | Required operand missing |
| `missing_operand_recovers.rns` | Missing operand + subsequent error |
| `outside_class_before.rns` | Directive before `.class` rejected |
| `*_as_name.rns` | Invalid tokens used as operands (access_flag, directive, label, numeric, string, type_hint) |
| `trailing_token(s).rns` | Extra tokens after directive |
| `trailing_token(s)_recovers.rns` | Trailing tokens + subsequent error |

### Recovery test pattern

Recovery tests prove the parser doesn't fail fast. After the primary error, add a line that triggers a second error:

```rns
.class public MyClass
.super java/lang/Object

.directive_name <error_condition>

     unknown_token ; <- this triggers E-007 (unexpected token in class body)

.class_end
```

The snapshot must show BOTH errors reported.

### Test file header

Every `.rns` test file needs a header comment:

```rns
; Tests that <specific condition>.
; Expected error(s): E-XXX
```

### Snapshot format

**Failed assembly** (error tests):
```
----- DISASSEMBLED -----
not generated
----- INPUT -----
<.rns file content>
----- STDERR -----
[<code>] Error: <message>
   ╭─[ <path>:<line>:<col> ]
   │
 N │ <source line>
   │ <ariadne labels>
   │
   │ Help: <suggestion>
   │ Note: <docs link>
───╯
----- JAVAP -----
not generated
```

**Successful assembly** (general/ tests):
```
----- DISASSEMBLED -----
<rnsc dis output>
----- INPUT -----
<.rns file content>
----- STDERR -----
(empty)
----- JAVAP -----
<javap -v -p output with <TEMP_DIR> and <DATE> normalized>
```

### Error codes

Defined in `rns-lang/src/diagnostic.rs`. Each code is unique.

| Code | Constant | Meaning |
|------|----------|---------|
| E-001 | ERR_CODE_UNCLOSED_IDENT | Unclosed quoted identifier |
| E-002 | ERR_CODE_UNKNOWN_DIR | Unknown directive |
| E-003 | ERR_CODE_TH_EXPECTS_NUM | Type hint expects numeric operand |
| E-004 | ERR_CODE_INVALID_ESCAPE | Invalid escape sequence in string |
| E-005 | ERR_CODE_INVALID_TYPE_HINT | Invalid/unknown type hint |
| E-006 | ERR_CODE_EMPTY_FILE | Empty file |
| E-007 | ERR_CODE_UNEXPECTED_TOKEN_IN_CLASS | Unexpected token inside class body |
| E-008 | ERR_CODE_TOKEN_OUTSIDE_CLASS | Token outside class definition |
| E-009 | ERR_CODE_IDENT_OF_TH_EXPECTED | Expected identifier or type hint |
| E-010 | ERR_CODE_CLASS_DEF_TRAILING_TOK | Trailing tokens after `.class` |
| E-011 | ERR_CODE_MULTIPLE_SUPER | Multiple `.super` directives |
| E-012 | ERR_CODE_SUPER_TRAILING_TOK | Trailing tokens after `.super` |
| E-013 | ERR_CODE_TH_TRAILING_TOK | Trailing tokens after type hint |
| E-014 | ERR_CODE_MISSING_TH_OPERAND | Missing type hint operand |
| E-015 | ERR_CODE_INVALID_CLASS_FLAG | Invalid access flag for class |
| E-016 | ERR_CODE_INVALID_METHOD_FLAG | Invalid access flag for method |
| E-017 | ERR_CODE_UNEXPECTED_TOKEN_IN_METHOD | Unexpected token inside method |
| E-018 | ERR_CODE_METHOD_TRAILING_TOK | Trailing tokens after `.method` |
| E-019 | ERR_CODE_MULTIPLE_CODE_DIR | Multiple `.code` directives in method |
| E-020 | ERR_CODE_MISSING_TH_IMPLICIT_OP | Missing operand (e.g. `.super` without class name) |
| E-021 | ERR_CODE_UNKNOWN_INSTRUCTION | Unknown bytecode instruction |
| E-022 | ERR_CODE_DIR_ATTR | Directive attribute error |
| E-023 | ERR_CODE_CLASS_END_TRAILING_TOK | Trailing tokens after `.class_end` |
| E-024 | ERR_CODE_INSTR_REQUIRES_EXPLICIT_TH | Instruction requires explicit type hint |
| E-025 | ERR_CODE_INSTR_TRAILING_TOK | Trailing tokens after instruction |
| E-026 | ERR_CODE_NOT_YET_IMPL | Not yet implemented |
| E-027 | ERR_CODE_UNDEFINED_LABEL | Undefined label reference |
| E-028 | ERR_CODE_NUMERIC_OPERAND_OVERFLOW | Numeric operand overflow |
| E-029 | ERR_CODE_PACKAGE_TRAILING_TOK | Trailing tokens after `.package` |
| E-030 | ERR_CODE_MULTIPLE_PACKAGE | Multiple `.package` directives |

### Warning codes

| Code | Tier | Meaning |
|------|------|---------|
| W-001 | AssemblerWarn | Missing `.super` directive (defaults to `java/lang/Object`) |
| W-002 | AssemblerWarn | Package name contains `.` (should use `/`) |
| JVMS-001 | JvmSpecWarn | JVM spec violation (e.g. interface without abstract) |

Diagnostic tiers: `SyntaxError` → always error. `AssemblerWarn`/`JvmSpecWarn` → warnings (suppressed by `-q`).

## Writing lexer tests

Lexer tests live in `rns-lang/test_data/unit/lexer/` with snapshots in `rns-lang/src/lexer/snapshots/`.

The test runner (`rns-lang/src/lexer/snapshot_tests.rs`) auto-discovers all `.rns` files. Snapshot format:

```
----- SOURCE -----
<input>
----- TOKENS -----
KIND                | SPAN    | LSP       | TEXT
----                | ----    | ---       | ----
DotClass            | 0..6    | 0:0..6    | .class
AccessFlag(public)  | 7..13   | 0:7..13   | public
Identifier("Test")  | 14..18  | 0:14..18  | Test
Newline             | 18..19  | 0:18..19  | \n
```

The test cross-validates byte-offset spans against line/col spans — both must extract the same text.

Lexer test directories: `comments/`, `complex/`, `directives/`, `dot/`, `identifiers/`, `init_clinit/`, `labels/`, `type_hints/`, `whitespace/`.

## Writing CLI tests

CLI tests live in `rnsc/test_data/cli_integration/`. Each fixture assembled twice (default + `-q`). Snapshot captures both runs side-by-side to verify warnings suppressed in quiet mode while errors remain.

## Conventions

- **Parser NEVER fails fast** — always continue parsing and report all errors
- Help messages suggest fixes. Note messages link to `https://rune.lagertha-vm.com/errors/<code>`.
- Package names use `/` separator in bytecode (`.package com/example/test`). Warning W-002 if `.` used.

## Adding a new directive checklist

1. Add token kind in `rns-lang/src/token/kind.rs`
2. Add lexer tests in `rns-lang/test_data/unit/lexer/directives/`
3. Update `all_directives_single_line.rns`, `all_directives_multi_line.rns`, `all_directives_with_code.rns`
4. Add parser error test matrix in `rnsc/test_data/rns_integration/error/parser/<directive>/`:
   - `<feature>.rns` + `<feature>_recovers.rns`
   - `missing_operand.rns` + `missing_operand_recovers.rns` (if applicable)
   - `multiple_definitions.rns` + `multiple_definitions_recovers.rns` (if applicable)
   - `trailing_tokens.rns` + `trailing_tokens_recovers.rns`
   - `*_as_name.rns` variants (error-producing go in `error/parser/`, warning-only go in `rns_warn/`)
5. Add `outside_class_before.rns` if directive must appear inside class body
6. When a directive can also appear inside `.inner` or `.inner_classes_attr` body, add the same test matrix for each context (e.g. `inner_directive/`, `inner_classes_attr_directive/`)
7. Suppress W-001 noise: always add `.super java/lang/Object` to test fixtures that declare classes (both `.class` and `.inner` blocks) unless the test is specifically about missing super
8. Run `cargo test --workspace` then `cargo insta review` to accept snapshots
