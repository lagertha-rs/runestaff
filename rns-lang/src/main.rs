use crate::lexer::JasmLexer;

mod cursor;
mod lexer;
mod token;

const KAKA: &str = "\
.class public HelloWorld
.super java/lang/Object

.method public static main([Ljava/lang/String;)V
    .limit stack 2
    ldc \"Hello, World!\"
    return
.end method
";

fn main() {
    let mut lexer = JasmLexer::new(KAKA);

    println!("{:?}", lexer.tokenize());
}
