use crate::assembler::program_parsers::program;
use crate::vm::VM;

use nom::types::CompleteStr;
use std;
use std::io;
use std::io::Write;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::num::ParseIntError;

/// Core structure for the REPL for the Assembler
pub struct REPL {
    vm: VM,
    command_buffer: Vec<String>,
}

impl REPL {
    pub fn new() -> REPL {
        REPL {
            vm: VM::new(),
            command_buffer: vec![],
        }
    }

    pub fn run(&mut self) {
        println!("Welcome to Iridium!");

        loop {
            let mut buffer = String::new();

            // Blocking call until the user types in a command
            let stdin = io::stdin();

            print!(">>> ");

            io::stdout().flush().expect("Unable to flush stdout");
            stdin
                .read_line(&mut buffer)
                .expect("Unable to read line from user");

            let buffer = buffer.trim();

            self.command_buffer.push(buffer.to_string());

            match buffer {
                ".quit" => {
                    println!("Farewell!");
                    std::process::exit(0);
                }
                ".history" => {
                    for command in &self.command_buffer {
                        println!("{}", command);
                    }
                }
                ".program" => {
                    println!("Listing instructions currently in VM's program vector:");
                    for instruction in &self.vm.program {
                        println!("{}", instruction);
                    }
                    println!("End of Program Listing");
                }
                ".registers" => {
                    println!("Listing registers and all contents:");
                    println!("{:#?}", self.vm.registers);
                    println!("End of Register Listing")
                }
                ".load_file" => {
                    print!("Please enter the path to the file you wish to load: ");
                    io::stdout().flush().expect("Unable to flush stdout");

                    let mut tmp = String::new();
                    stdin.read_line(&mut tmp).expect("Unable to read line from user");

                    let tmp = tmp.trim();
                    let filename = Path::new(&tmp);
                    let mut f = File::open(Path::new(&filename)).expect("File not found");
                    let mut contents = String::new();
                    
                    f.read_to_string(&mut contents).expect("There was an error reading from the file");

                    let program = match program(CompleteStr(&contents)) {
                        Ok((remainder, program)) => {
                            program
                        }
                        Err(e) => {
                            println!("Unable to parse input: {:?}", e);
                            continue;
                        }
                    };

                    self.vm.program.append(&mut program.to_bytes());
                }
                _ => {
                    let parsed_program = program(CompleteStr(buffer));

                    if !parsed_program.is_ok() {
                        println!("Unable to parse input");
                        continue;
                    }

                    let (_, result) = parsed_program.unwrap();
                    let bytecode = result.to_bytes();

                    // TODO: Make a function to let us add bytes to the VM
                    for byte in bytecode {
                        self.vm.add_byte(byte);
                    }

                    self.vm.run_once();
                }
            }
        }
    }
}
