# Runestaff - Java Bytecode Assembler/Disassembler

## Project Overview

Runestaff is a Rust-based Java bytecode assembler and disassembler that converts between human-readable `.rns` (Rune Assembly) files and Java `.class` bytecode files. It's designed to generate arbitrary bytecode, including invalid or unusual bytecode that `javac` would never produce, making it useful for testing JVM implementations.

The project consists of three main crates:
- **rns-lang**: Core library for parsing, assembling, and disassembling
- **rnsc**: CLI tool for assembling and disassembling
- **rns-lsp**: Language Server Protocol implementation

## Test Approach

### Integration Test Structure

All integration tests are located in `rnsc/test_data/rns_integration/` and follow a strict pattern:

```
test_data/rns_integration/
├── error/
│   ├── lexer/           # Lexer error tests
│   └── parser/          # Parser error tests
│       ├── class_body/
│       ├── class_header/
│       ├── code_header/
│       ├── instructions/
│       ├── method_directive/
│       ├── outside_class_directive/
│       ├── package_directive/
│       ├── super_directive/
│       └── type_hints/
├── general/             # Valid bytecode tests
├── jvms/                # JVM spec compliance tests
└── rns_warn/            # Warning tests
```

### Lexer Unit Tests

Lexer tests are located in `rns-lang/test_data/unit/lexer/` and verify tokenization:

```
test_data/unit/lexer/
├── comments/          # Comment handling
├── complex/           # Complex real-world examples
├── directives/        # All directive tokens (.class, .super, .method, .package, etc.)
├── dot/               # Bare dot and dot-related edge cases
├── identifiers/       # Identifier tokenization (quoted, unquoted, special chars)
├── init_clinit/       # <init> and <clinit> special methods
├── labels/            # Label tokenization
├── type_hints/        # Type hint tokenization (@class, @string, etc.)
└── whitespace/        # Whitespace handling
```

Lexer tests use snapshot testing to verify:
- Token kinds are correctly identified
- Token spans (byte offsets and line/column positions) are accurate
- Text extraction from spans matches the source

**Snapshot format:**
```
----- SOURCE -----
<input source>
----- TOKENS -----
KIND    | SPAN    | LSP     | TEXT
----    | ----    | ---     | ----
...
```

### CLI Integration Tests

CLI tests are located in `rnsc/test_data/cli_integration/` and verify command-line behavior:

```
test_data/cli_integration/
└── quiet/             # Tests for -q/--quiet flag
```

CLI tests run each `.rns` fixture twice:
1. Default mode (no flags)
2. Quiet mode (`-q`)

**Snapshot format:**
```
----- INPUT -----
<input source>
----- DEFAULT -----
stdout: <output>
stderr: <diagnostics>
exit: <true/false>
hash: <sha256>
----- QUIET (-q) -----
stdout: <output>
stderr: <diagnostics>
exit: <true/false>
hash: <sha256>
```

This allows verifying that warnings are suppressed in quiet mode while errors remain.

### Test Naming Convention

For each directive or feature, we maintain a comprehensive test suite with these variants:

1. **Basic error test** (`<feature>.rns`)
   - Tests the primary error condition
   - Example: `trailing_token.rns` - tests trailing tokens after directive

2. **Recovery test** (`<feature>_recovers.rns`)
   - Tests that parser continues after error and reports subsequent errors
   - Contains the primary error PLUS an additional error that should also be reported
   - Example: `trailing_token_recovers.rns` - trailing tokens + unknown token after

3. **Multiple definitions test** (`multiple_definitions.rns`)
   - Tests error when directive appears multiple times
   - Example: `multiple_definitions.rns` - two `.super` directives

4. **Multiple definitions with recovery** (`multiple_definitions_recovers.rns`)
   - Multiple definitions error + subsequent error
   - Example: `multiple_definitions_recovers.rns` - two `.super` + unknown token

5. **Missing operand test** (`missing_operand.rns`)
   - Tests error when required operand is missing
   - Example: `missing_operand.rns` - `.super` without class name

6. **Missing operand with recovery** (`missing_operand_recovers.rns`)
   - Missing operand error + subsequent error
   - Example: `missing_operand_recovers.rns` - `.super` without name + unknown token

7. **Before class directive test** (`outside_class_before.rns` or `err_before_dot_class.rns`)
   - Tests that directive appearing before `.class` is rejected
   - Example: `outside_class_before.rns` - `.package` before `.class`

8. **Edge case tests** (`*_as_name.rns`, `directive_as_name_*.rns`)
   - Tests invalid tokens used as operands
   - Examples: `access_flag_as_name.rns`, `directive_as_name_class.rns`

### Test File Structure

Every test file MUST follow this structure:

```rns
; Brief description of what this test validates
; Expected error code(s): E-XXX, E-YYY
; This proves:
;   1. Parser detects the specific error condition
;   2. Error message is clear and helpful
;   3. [Additional validation points]

.class public MyClass
.super java/lang/Object

; The directive being tested with the error condition
.directive_name <error_condition>

.class_end
```

For recovery tests:

```rns
; Tests that parser continues after <error> and reports subsequent errors
; Expected errors: E-XXX (primary), E-YYY (recovery)

.class public MyClass
.super java/lang/Object

.directive_name <error_condition>

     unknown_token ; <- still reported after recovery

.class_end
```

### Snapshot Format

Snapshots are stored in `rnsc/tests/snapshots/` and contain:

```
----- DISASSEMBLED -----
<disassembled output or "not generated">
----- INPUT -----
<original .rns file content>
----- STDERR -----
<error/warning output>
----- JAVAP -----
<javap -v -p output for successful compilations>
```

For successful compilations, we use `javap -v -p` output instead of hash to provide more meaningful verification.

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test category
cargo test --test rns_test

# Accept new snapshots after changes
cargo insta accept

# Review snapshot changes
cargo insta review
```

## Code Style Guidelines

### Comments

- Every test file must have a header comment explaining what it tests
- Complex logic should have inline comments
- Error messages should be clear and actionable

### Error Handling

- Parser should NEVER fail fast - always continue and report all errors
- Each error should have a unique error code (E-XXX)
- Each warning should have a unique warning code (W-XXX)
- Help messages should suggest fixes
- Note messages should link to documentation

### Package Directive Specifics

- Package names are kept exactly as written in bytecode (no automatic conversion)
- Users should use `/` separator directly (e.g., `.package com/example/test`)
- Warning W-002 is shown when `.` is used in package name to suggest using `/` instead
- File system directories are created based on the package name as written

## Testing Checklist

When adding a new directive or feature, ensure you have:

### Parser/Assembler Tests (rnsc/test_data/rns_integration/)
- [ ] Basic error test for each error condition
- [ ] Recovery test for each error condition
- [ ] Multiple definitions test (if applicable)
- [ ] Multiple definitions with recovery (if applicable)
- [ ] Missing operand test (if applicable)
- [ ] Missing operand with recovery (if applicable)
- [ ] Before class directive test
- [ ] Edge case tests (invalid tokens as operands)
- [ ] Header comments in all test files
- [ ] Snapshots use javap output for successful compilations
- [ ] All tests pass with `cargo test`
- [ ] No clippy warnings with `cargo clippy`

### Lexer Tests (rns-lang/test_data/unit/lexer/)
- [ ] Directive tokenization test (add to directives/ directory)
- [ ] Update all_directives_*.rns files if new directive added
- [ ] Verify token spans are correct
- [ ] Verify text extraction matches source

### CLI Tests (rnsc/test_data/cli_integration/)
- [ ] Test with warnings (verify quiet mode suppresses them)
- [ ] Test with errors (verify quiet mode doesn't suppress them)
- [ ] Verify hash is consistent between runs
