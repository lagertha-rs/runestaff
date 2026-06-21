# Runestaff

RNS (Rune Assembly) is a human-readable assembly language that compiles to JVM bytecode. It provides a clean syntax for defining classes, methods, and bytecode instructions.

Example:

```rns
.class public HelloWorld
.super java/lang/Object

.method public <init> ()V
  .code stack 1 locals 1
    aload_0
    invokespecial java/lang/Object <init> ()V
    return
  .code_end
.method_end

.method static main ([Ljava/lang/String;)V
  .code stack 2 locals 1
    getstatic java/lang/System out Ljava/io/PrintStream;
    ldc @string "Hello World!"
    invokevirtual java/io/PrintStream println (Ljava/lang/String;)V
    return
  .code_end
.method_end
.class_end
```

Originally created to test the [Lagertha VM](https://github.com/lagertha-rs/lagertha), RNS has evolved into a standalone toolchain for JVM bytecode generation.

## Current Status (v0.1)

### Implemented

**Lexer**
- All token types: directives, access flags, type hints, identifiers (bare + quoted), labels, comments
- Error recovery: skips to end of line on error, continues tokenizing
- "Did you mean?" suggestions for unknown directives and type hints

**Parser**
- Class declaration with access flags (17 flags supported)
- `.super` directive for superclass
- `.method` directive with name, descriptor, access flags
- `.code` directive with stack/locals specification
- Type hints: `@utf8`, `@int`, `@string`, `@class`, `@methodref`, `@fieldref`, `@cp_idx`
- Error recovery with anchor-based fallback
- Comprehensive error messages with source location and suggestions

**Assembler**
- Generates JVM class files (Java 5 format)
- Constant pool construction for: `Utf8`, `Class`, `Integer`, `String`, `Methodref`, `Fieldref`
- Label resolution for branch targets
- Bytecode emission for supported instructions

**Instructions (12 of ~200)**
- No operand: `aload_0`, `iload_0`, `iconst_0`, `return`
- Method reference: `invokespecial`, `invokevirtual`, `invokestatic`
- Field reference: `getstatic`
- Type hint: `ldc`
- Byte operand: `bipush`
- Label operand: `goto`, `if_icmpeq`

**CLI (rnsc)**
- `rnsc <file.rns>` — assemble to `.class` file
- `rnsc asm <file.rns> [-o output.class]` — assemble with explicit output
- `rnsc dis <file.class>` — disassemble to RNS syntax

**Disassembler**
- Parses class files and outputs RNS syntax
- Handles constant pool references
- Formats class/method flags
- **Note**: Output uses `.end class`/`.end method`/`.end code` (incompatible with assembler, needs fix)

**Testing**
- 205 integration tests passing
- Snapshot-based error case coverage
- Lexer unit tests with comprehensive edge cases

### Not Implemented

- **Field directives** — cannot define class fields
- **~190 JVM instructions** — missing most arithmetic, stack manipulation, array operations, etc.
- **Type hints**: `@float`, `@long`, `@double`, `@interfacemethodref`, `@nametype`, `@methodhandle`, `@methodtype`, `@dynamic`, `@invokedynamic`
- **Exception handling** — no `.catch`/`.try` directives
- **Attributes** — no LineNumberTable, LocalVariableTable, StackMapTable
- **Annotations** — tokenized but not parsed
- **Parser unit tests** — all commented out (869 lines)
- **Stack/locals auto-calculation** — must be manually specified

## Roadmap

### v0.2 — Essentials for Real Programs

- [ ] Expand instruction set to ~50 most common:
  - All `xload`/`xstore`/`xconst` variants (`iload`, `istore`, `lload`, `lstore`, `fload`, `fstore`, `dload`, `dstore`, `aload`, `astore`, `lconst`, `fconst`, `dconst`, `aconst_null`)
  - All `xreturn` variants (`ireturn`, `lreturn`, `freturn`, `dreturn`, `areturn`)
  - Arithmetic: `iadd`, `isub`, `imul`, `idiv`, `irem`, `ineg`, `ladd`, `lsub`, `lmul`, `ldiv`, `lrem`, `lneg`, `fadd`, `fsub`, `fmul`, `fdiv`, `frem`, `fneg`, `dadd`, `dsub`, `dmul`, `ddiv`, `drem`, `dneg`
  - Stack: `dup`, `dup_x1`, `dup_x2`, `dup2`, `swap`, `pop`, `pop2`
  - Field access: `putstatic`, `getfield`, `putfield`
  - Object: `new`
  - All `if_*` branches (`ifeq`, `ifne`, `iflt`, `ifge`, `ifgt`, `ifle`, `if_icmpne`, `if_icmplt`, `if_icmpge`, `if_icmpgt`, `if_icmple`, `ifnull`, `ifnonnull`)
- [ ] Add `.field` directive for class fields
- [ ] Replace all `unimplemented!()` with proper error messages
- [ ] Replace production `.unwrap()` with error handling
- [x] Fix disassembler output to match assembler syntax (`.end class` → `.class_end`, `strictfp` → `strict`)
- [ ] Restore parser unit tests (uncomment and update to current API)
- [x] Fix `ldc` without type hint (currently `todo!()`)
- [ ] Max test coverage for all
- [ ] Extract all "jvm specification errors" analyze to the jclass crate to make it reusable between RNS and VM

### v0.3 — Complete JVM Coverage

- [ ] Full JVM instruction set (~200 instructions)
- [ ] Exception handling (`.catch`/`.try` directives, exception tables)
- [ ] Attributes: LineNumberTable, LocalVariableTable, StackMapTable, SourceFile
- [ ] Stack/locals auto-calculation
- [ ] Configurable class file version (currently hardcoded to Java 5)
- [ ] Annotation support
- [ ] Assembler unit tests
- [ ] LSP improvements (parser diagnostics, completion, hover)
- [ ] Complete type hint support (all 18 variants)

## Crates

- **rns-lang** — RNS language parser, compiler, and assembler library
- **rnsc** — RNS assembler/compiler CLI
- **rns-lsp** — Language Server Protocol implementation for RNS (Not usable, just a POC)

## Dependencies

This workspace depends on [lvm-class](https://github.com/lagertha-rs/lagertha) (JVM class file parser and writer) from the Lagertha monorepo.
