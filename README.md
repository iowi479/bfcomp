# JIT Compiler for Brainfuck

This Implementation enables executing of Brainfuckprograms via a Interpreter or a JIT-Compiler.

## Usage

```console
cargo run bfcomp {jit | int} <file_path> 
```

## Examples

JIT-Compiler

```console
cargo run jit examples/hello_world.bf
```

Interpreter

```console
cargo run int examples/hello_world.bf
```

## Source

Idea and context: [Tsoding](https://www.youtube.com/watch?v=mbFY3Rwv7XM)
