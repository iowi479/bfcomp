use std::collections::HashMap;
use std::fmt::{Display, Error, Formatter};
use std::io::{stdin, Read};
use std::str::Chars;

const JIT_MEMORY_SIZE: usize = 10 * 1024; // Default = 1KB

enum Instruction {
    Add(u8),
    Sub(u8),
    Left(usize),
    Right(usize),
    Input(usize),
    Output(usize),
    JumpIfZero(usize),
    JumpIfNotZero(usize),
}

pub struct BFProgram {
    instructions: Vec<Instruction>,
}

struct BFSourceCode<'a> {
    chars: Chars<'a>,
}

/// The contained byte code is executable and can be called with a pointer to a memory slice.
///
/// If the memory goes out of scope, the executable will segfault.
/// Since the byte code is deallocated.
struct BFExecutable {
    /// The provided pointer is used as the memory while executing the byte code.
    /// This has to be sized appropriately since there are no runtime checks.
    executable: fn(*mut [u8]),

    /// This contains the byte code for the executable.
    #[allow(unused)]
    source: memmap2::Mmap,
}

impl BFProgram {
    /// This parses the provided source code into a usable BFProgram.
    pub fn parse_program(source_code: &str) -> BFProgram {
        let mut source_code = BFSourceCode {
            chars: source_code.chars(),
        };
        return source_code.parse_program();
    }

    pub fn execute_with_interpreter(&self) {
        let mut ip: usize = 0;
        let mut mp: usize = 0;
        let mut memory: Vec<u8> = vec![0; 64];

        while ip < self.instructions.len() {
            match self.instructions[ip] {
                Instruction::Add(count) => {
                    memory[mp] = memory[mp].overflowing_add(count).0;
                    ip += 1;
                }
                Instruction::Sub(count) => {
                    memory[mp] = memory[mp].overflowing_sub(count).0;
                    ip += 1;
                }
                Instruction::Left(count) => {
                    assert!(mp >= count);
                    mp -= count;
                    ip += 1;
                }
                Instruction::Right(count) => {
                    mp += count;
                    if mp >= memory.len() {
                        memory.reserve(mp + 1);
                    }
                    ip += 1;
                }
                Instruction::Input(count) => {
                    for _ in 0..count {
                        let mut buf: [u8; 1] = [0];
                        let result = stdin().read(&mut buf);
                        if result.is_ok() && result.ok().unwrap() == 1 {
                            memory[mp] = buf[0];
                        } else {
                            panic!("Error reading input");
                        }
                    }
                    ip += 1;
                }
                Instruction::Output(count) => {
                    for _ in 0..count {
                        print!("{}", memory[mp] as char);
                    }
                    ip += 1;
                }
                Instruction::JumpIfZero(dest) => {
                    if memory[mp] == 0 {
                        ip = dest;
                    } else {
                        ip += 1;
                    }
                }
                Instruction::JumpIfNotZero(dest) => {
                    if memory[mp] != 0 {
                        ip = dest;
                    } else {
                        ip += 1;
                    }
                }
            }
        }
    }

    pub fn execute_with_jit_compiler(&self) {
        let byte_code = self.jit_compile();

        match BFExecutable::make_executable(&byte_code) {
            Ok(executable) => {
                let mut memory: [u8; JIT_MEMORY_SIZE] = [0; JIT_MEMORY_SIZE];
                executable.execute(&mut memory);
            }
            Err(e) => {
                panic!("Error making compiled code executable: {}", e);
            }
        }
    }

    fn jit_compile(&self) -> Vec<u8> {
        let mut byte_code: Vec<u8> = Vec::new();

        let mut jump_addresses: HashMap<usize, usize> = HashMap::new();
        let mut backpatch_addresses: HashMap<usize, usize> = HashMap::new();

        for (i, instruction) in self.instructions.iter().enumerate() {
            let mut instruction_code = match instruction {
                Instruction::Add(count) => {
                    vec![0x80, 0x07, *count] // add byte [rdi], count
                }

                Instruction::Sub(count) => {
                    vec![0x80, 0x2F, *count] // sub byte [rdi], count
                }

                Instruction::Right(count) => {
                    let steps = *count as u32;
                    let b = steps.to_le_bytes();
                    vec![0x48, 0x81, 0xC7, b[0], b[1], b[2], b[3]] // add rdi, count
                }

                Instruction::Left(count) => {
                    let steps = *count as u32;
                    let b = steps.to_le_bytes();
                    vec![0x48, 0x81, 0xEF, b[0], b[1], b[2], b[3]] // sub rdi, count
                }

                Instruction::Output(count) => {
                    let mut code: Vec<u8> = Vec::new();
                    for _ in 0..*count {
                        code.append(
                            vec![
                                0x57, // push rdi
                                0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00, // mov rax, 1
                                0x48, 0x89, 0xfe, // mov rsi, rdi
                                0x48, 0xc7, 0xc7, 0x01, 0x00, 0x00, 0x00, // mov rdi, 1
                                0x48, 0xc7, 0xc2, 0x01, 0x00, 0x00, 0x00, // mov rdx, 1
                                0x0f, 0x05, // syscall
                                0x5f, // pop rdi
                            ]
                            .as_mut(),
                        );
                    }
                    code
                }

                Instruction::Input(count) => {
                    let mut code: Vec<u8> = Vec::new();
                    for _ in 0..*count {
                        code.append(
                            vec![
                                0x57, // push rdi
                                0x48, 0xc7, 0xc0, 0x00, 0x00, 0x00, 0x00, // mov rax, 0
                                0x48, 0x89, 0xfe, // mov rsi, rdi
                                0x48, 0xc7, 0xc7, 0x00, 0x00, 0x00, 0x00, // mov rdi, 0
                                0x48, 0xc7, 0xc2, 0x01, 0x00, 0x00, 0x00, // mov rdx, 1
                                0x0f, 0x05, // syscall
                                0x5f, // pop rdi
                            ]
                            .as_mut(),
                        );
                    }
                    code
                }

                Instruction::JumpIfZero(dest) => {
                    let code = vec![
                        0x48, 0x31, 0xc0, // xor rax, rax
                        0x8a, 0x07, // mov al, byte [rdi]
                        0x48, 0x85, 0xc0, // test rax, rax
                        0x0f, 0x84, 0x00, 0x00, 0x00, 0x00, // je <placeholder-dest>
                    ];

                    let current_byte_address = byte_code.len() + code.len();
                    jump_addresses.insert(i + 1, current_byte_address);
                    backpatch_addresses.insert(*dest, current_byte_address - 4);

                    code
                }

                Instruction::JumpIfNotZero(dest) => {
                    let dst_address = jump_addresses.get(dest);
                    assert!(dst_address.is_some());
                    let dst_address = dst_address.unwrap();

                    let mut code = vec![
                        0x48, 0x31, 0xc0, // xor rax, rax
                        0x8a, 0x07, // mov al, byte [rdi]
                        0x48, 0x85, 0xc0, // test rax, rax
                    ];

                    let current_address = byte_code.len() + code.len() + 6;
                    let offset: u32 = (dst_address.overflowing_sub(current_address).0) as u32;
                    let b = offset.to_le_bytes();
                    code.append(vec![0x0f, 0x85, b[0], b[1], b[2], b[3]].as_mut()); // jne <dest>
                    jump_addresses.insert(i + 1, byte_code.len() + code.len());

                    code
                }
            };

            byte_code.append(&mut instruction_code);
        }

        // Backpatching
        for (dest_instruction, source_location) in backpatch_addresses.iter() {
            let dest_address = jump_addresses.get(dest_instruction).unwrap();
            let offset = dest_address - (source_location + 4); // after 4 bytes of jump-address
            let b = offset.to_le_bytes();
            byte_code[*source_location] = b[0];
            byte_code[*source_location + 1] = b[1];
            byte_code[*source_location + 2] = b[2];
            byte_code[*source_location + 3] = b[3];
        }

        byte_code.push(0xC3); // ret

        return byte_code;
    }
}

impl BFSourceCode<'_> {
    fn parse_program(&mut self) -> BFProgram {
        let mut instructions: Vec<Instruction> = Vec::new();
        let mut jump_stack: Vec<usize> = Vec::new();
        let mut current_char = self.next();

        loop {
            if current_char.is_none() {
                break;
            }

            match current_char {
                Some('[') => {
                    jump_stack.push(instructions.len());
                    instructions.push(Instruction::JumpIfZero(0));
                    current_char = self.next();
                }
                Some(']') => {
                    let jump_if_zero = jump_stack.pop().expect("Stack underflow at {current_char}");
                    instructions.push(Instruction::JumpIfNotZero(jump_if_zero + 1));

                    let jump_if_not_zero = instructions.len();
                    instructions[jump_if_zero] = Instruction::JumpIfZero(jump_if_not_zero);
                    current_char = self.next();
                }

                Some(c) => {
                    let mut count: usize = 1;
                    let mut next_char = self.next();
                    while next_char == Some(c) {
                        count += 1;
                        next_char = self.next();
                    }

                    match c {
                        '+' => {
                            assert!(count < 256);
                            instructions.push(Instruction::Add(count as u8))
                        }
                        '-' => {
                            assert!(count < 256);
                            instructions.push(Instruction::Sub(count as u8))
                        }
                        '<' => instructions.push(Instruction::Left(count)),
                        '>' => instructions.push(Instruction::Right(count)),
                        ',' => instructions.push(Instruction::Input(count)),
                        '.' => instructions.push(Instruction::Output(count)),
                        _ => panic!("Invalid character"),
                    }
                    current_char = next_char;
                }

                None => break,
            }
        }

        return BFProgram { instructions };
    }
}

impl BFExecutable {
    /// Moves the provided byte code into a memory map and makes it executable.
    /// Returns a executable function pointer to the byte code.
    fn make_executable(byte_code: &Vec<u8>) -> Result<BFExecutable, std::io::Error> {
        let mut mem = memmap2::MmapOptions::new()
            .len(byte_code.len())
            .map_anon()?;
        mem.copy_from_slice(byte_code);
        let mem = mem.make_exec()?;
        let f: fn(*mut [u8]) = unsafe { std::mem::transmute(mem.as_ptr()) };

        return Ok(BFExecutable {
            executable: f,
            source: mem,
        });
    }

    fn execute(&self, memory: &mut [u8]) {
        (self.executable)(memory);
    }
}

impl<'a> Iterator for BFSourceCode<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(c) = self.chars.next() {
            match c {
                '+' | '-' | '<' | '>' | ',' | '.' | '[' | ']' => return Some(c),
                _ => continue,
            }
        }
        return None;
    }
}

impl Display for BFProgram {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        for (i, instruction) in self.instructions.iter().enumerate() {
            write!(f, "{i}: {instruction}\n")?;
        }
        Ok(())
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            Instruction::Add(count) => write!(f, "Add({})", count),
            Instruction::Sub(count) => write!(f, "Sub({})", count),
            Instruction::Left(count) => write!(f, "Left({})", count),
            Instruction::Right(count) => write!(f, "Right({})", count),
            Instruction::Input(count) => write!(f, "Input({})", count),
            Instruction::Output(count) => write!(f, "Output({})", count),
            Instruction::JumpIfZero(count) => write!(f, "JumpIfZero({})", count),
            Instruction::JumpIfNotZero(count) => write!(f, "JumpIfNotZero({})", count),
        }
    }
}
