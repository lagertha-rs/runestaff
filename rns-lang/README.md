# runestaff

Java bytecode assembler and disassembler.

## Name

As it is the side project of the LagerthaVM (named after my cat), I wanted to keep the name related to the main
project.
`Runestaff` is an old word meaning a letter in runic alphabets, which is a nice fit for a project that assembles and
disassembles bytecode.

## Errors and warnings

Runestaff will report errors and warnings when it encounters issues in the assembly code. Runestaff is designed to allow
produce
any bytecode, even if it is invalid. This means that Runestaff will not prevent you from writing code that may not work.

Hovewer, I don't want to leave you in the dark about potential issues in your code, even if they are intentional.
Runestaff will report errors and warnings to help you identify and fix potential problems in your code.