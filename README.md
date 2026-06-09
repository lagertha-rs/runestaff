# Runestaff

RNS language toolchain for the [Lagertha VM](https://github.com/lagertha-rs/lagertha) project.

## Crates

- **rns-lang** — RNS language parser, compiler, and assembler library
- **rnsc** — RNS assembler/compiler CLI
- **rns-lsp** — Language Server Protocol implementation for RNS

## Dependencies

This workspace depends on [lvm-class](https://github.com/lagertha-rs/lagertha) (JVM class file parser and writer) from the Lagertha monorepo.

## Development

```bash
cargo build --workspace
cargo test --workspace
```
