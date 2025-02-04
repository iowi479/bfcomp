# JIT Compiler for Brainfuck

This Implementation enables executing of Brainfuckprograms via a Interpreter or a JIT-Compiler.

The JIT-Compiler is only tested in an Ubuntu-WSL2-Instance on a x86-64 machine.
Other Systems may not work!

## Usage

```console
bfcomp {jit | int} <file_path> 
```

## Examples

JIT-Compiler

```console
bfcomp jit examples/hello_world.bf
```

Interpreter

```console
bfcomp int examples/hello_world.bf
```

## Source

Idea and context: [Tsoding Stream](https://www.youtube.com/watch?v=mbFY3Rwv7XM)
