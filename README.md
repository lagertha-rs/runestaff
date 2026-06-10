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

## Crates

- **rns-lang** — RNS language parser, compiler, and assembler library
- **rnsc** — RNS assembler/compiler CLI
- **rns-lsp** — Language Server Protocol implementation for RNS (Not usable, just a POC)

## Dependencies

This workspace depends on [lvm-class](https://github.com/lagertha-rs/lagertha) (JVM class file parser and writer) from the Lagertha monorepo.
