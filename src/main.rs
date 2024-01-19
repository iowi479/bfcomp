use bfcomp::BFProgram;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        println!("Usage: bfcomp {{jit | int}} <file_path>");
        println!("Example: bfcomp jit hello_world.bf");
        println!(" - jit: Just in time compile the program and execute it");
        println!(" - int: Interpret the program\n");
        panic!("Two arguments required");
    }

    let mode = &args[1];
    let file_path = &args[2];

    if mode != "jit" && mode != "int" {
        panic!("Invalid mode");
    }

    let contents =
        std::fs::read_to_string(file_path).expect("Something went wrong reading the file");

    let program = BFProgram::parse_program(&contents);

    println!("Brainfuck program Output:");
    match mode.as_str() {
        "jit" => program.execute_with_jit_compiler(),
        "int" => program.execute_with_interpreter(),
        _ => panic!("Invalid mode"),
    }
    println!(" -> Exited with code 0");
}
