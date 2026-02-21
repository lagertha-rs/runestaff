# Test Redesign Specification (COMPLETED)

## Final_data/
└ Structure

```
test── integration/           # All test cases in one place
    ├── error/            # Error test cases (105 files)
    │   └── err/         # lexer/parser errors
    └── general/         # Roundtrip test cases (2 files)
        └── *.ja

tests/
└── jasm_test.rs         # Single unified test file

snapshots/
└── *.snap               # 321 snapshots (test × flag variants)
```

## Test Execution

- 321 total tests (107 test files × 3 flag variants: "", "-Wasm", "-Werror")
- Single `#[rstest]` with parameterized flags
- All tests use the same format: DISASSEMBLED → INPUT → STDERR → HASH

## Notes

- CLI flags `--wasm` and `--werror` are accepted but logic not implemented
- Uses SHA256 hash instead of full hexdump
- "not generated" shown when assembler fails

## 1. CLI Flags for Warning Levels

| Flag | Behavior |
|------|----------|
| (none) | Show warnings, continue assembly (level 0) |
| `-Wasm` | Assembler warnings → errors, jvm warnings → warnings |
| `-Werror` | All warnings → errors (strict mode) |

### Implementation Notes
- Add CLI flags parsing (accept but don't implement logic yet)
- `-Wjvm` can be alias for `-Werror`

## 2. Unified Test File

### Merge jasm_error_test.rs + roundtrip_test.rs → jasm_test.rs

Single test file handles both error and roundtrip cases.

### New Snapshot Format

```
----- DISASSEMBLED -----
<class disassembly or "not generated">

----- INPUT -----
<source code>

----- STDERR -----
<warnings/errors>

----- HASH -----
<sha256 hash of classfile or "not generated">
```

### Key Points
- When assembler fails (error), print "not generated" in DISASSEMBLED and HASH sections
- Use SHA256 hash instead of full hexdump for smaller snapshots
- Order: DISASSEMBLED → INPUT → STDERR → HASH

### Pre-insta Assertion
- Assert `stdout` is empty
- If not empty, print the stdout content and fail the test

## 3. Test Data Organization

```
test_data/
├── error/                    # jasm syntax errors (snapshots: err-*.snap)
│   └── err/
├── roundtrip/                # assemble -> disassemble (snapshots: roundtrip-*.snap)
│   └── *.ja
└── unit/
```

## 4. Snapshot Naming

- Error tests: `err-*.snap`
- Roundtrip tests: `roundtrip-*.snap`

## 5. Implementation Steps

1. Add CLI flags `-Wasm`, `-Werror` (accept but don't implement logic)
2. Replace hex crate with sha2 for hashing
3. Merge test files into single `jasm_test.rs`
4. Update snapshot format to use HASH instead of HEXDUMP
5. Handle "not generated" case when assembler fails
6. Move all snapshots to single `snapshots/` directory
