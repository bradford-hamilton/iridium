use crate::instruction::Opcode;
use crate::assembler::program_parsers::program;
use crate::assembler::instruction_parsers::AssemblerInstruction;
use crate::assembler::assembler_errors::AssemblerError;
use crate::assembler::program_parsers::Program;

use nom::types::CompleteStr;

pub mod instruction_parsers;
pub mod opcode_parsers;
pub mod operand_parsers;
pub mod program_parsers;
pub mod register_parsers;
pub mod label_parsers;
pub mod assembler_errors;

pub const PIE_HEADER_PREFIX: [u8; 4] = [45, 50, 49, 45];
pub const PIE_HEADER_LENGTH: usize = 64;

#[derive(Debug, PartialEq)]
pub enum Token {
    Op { code: Opcode },
    Register { reg_num: u8 },
    IntegerOperand { value: i32 },
    LabelDeclaration { name: String },
    LabelUsage { name: String },
    Directive { name: String },
    IrString { name: String },
}

pub struct Symbol {
    name: String,
    offset: u32,
    symbol_type: SymbolType,
}

impl Symbol {
    pub fn new(name: String, symbol_type: SymbolType, offset: u32) -> Symbol {
        Symbol {
            name,
            symbol_type,
            offset,
        }
    }
}

pub enum SymbolType {
    Label,
}

pub struct SymbolTable {
    symbols: Vec<Symbol>,
}

impl SymbolTable {
    pub fn new() -> SymbolTable {
        SymbolTable { symbols: vec![] }
    }

    pub fn add_symbol(&mut self, s: Symbol) {
        self.symbols.push(s);
    }

    pub fn symbol_value(&self, s: &str) -> Option<u32> {
        for symbol in &self.symbols {
            if symbol.name == s {
                return Some(symbol.offset);
            }
        }
        None
    }
}

#[derive(Debug, Default)]
pub struct Assembler {
    /// Tracks which phase the assember is in
    phase: AssemblerPhase,
    /// Symbol table for constants and variables
    pub symbols: SymbolTable,
    /// The read-only data section constants are put in
    pub ro: Vec<u8>,
    /// The compiled bytecode generated from the assembly instructions
    pub bytecode: Vec<u8>,
    /// Tracks the current offset of the read-only section
    ro_offset: u32,
    /// A list of all the sections we've seen in the code
    sections: Vec<AssemblerSection>,
    /// The current section the assembler is in
    current_section: Option<AssemblerSection>,
    /// The current instruction the assembler is converting to bytecode
    current_instruction: u32,
    /// Any errors we find along the way. At the end, we'll present them to the user.
    errors: Vec<AssemblerError>
}

impl Assembler {
    pub fn new() -> Assembler {
        Assembler {
            current_instruction: 0,
            ro_offset: 0,
            ro: vec![],
            bytecode: vec![],
            sections: vec![],
            errors: vec![],
            phase: AssemblerPhase::First,
            symbols: SymbolTable::new(),
            current_section: None,
        }
    }

    pub fn assemble(&mut self, raw: &str) -> Result<Vec<u8>, Vec<AssemblerError>> {
        match program(CompleteStr(raw)) {
            Ok((_remainder, program)) => {
                let mut assembled_program = self.write_pie_header();
                self.process_first_phase(&program);

                if !self.errors.is_empty() {
                    return Err(self.errors.clone());
                }

                if self.sections.len() != 2 {
                    println!("Did not find at least two sections.");
                    self.errors.push(AssemblerError::InsufficientSections);
                    return Err(self.errors.clone());
                }

                let mut body = self.process_second_phase(&program);
                assembled_program.append(&mut body);
                Ok(assembled_program)
            }
            Err(e) => {
                println!("There was an error parsing the code: {:?}", e);
                Err(vec![AssemblerError::ParseError{ error: e.to_string() }])
            }
        }
    }

    /// Runs the first pass of the two-pass assembling process. It looks for labels and puts them in the symbol table
    fn process_first_phase(&mut self, p: &Program) {
        for i in &p.instructions {
            if i.is_label() {
                if self.current_instruction.is_some() {
                    self.process_label_declaration(&i);
                } else {
                    self.errors.push(
                        AssemblerError::NoSegmentDeclarationFound {
                            instruction: self.current_instruction,
                        }
                    )
                }
            }

            if i.is_directive() {
                self.process_directive(i);
            }

            self.current_instruction += 1;
        }

        self.phase = AssemblerPhase::Second;
    }

    /// Handles the declaration of a label such as: hello: .asciiz 'Hello'
    fn process_label_declaration(&mut self, i: &AssemblerInstruction) {
        let name = match i.get_label_name() {
            Some(name) => { name },
            None => {
                self.errors.push(AssemblerError::StringConstantDeclaredWithoutLabel {
                    instruction: self.current_instruction,
                });
                return;
            }
        };

        if self.symbols.has_symbol(&name) {
            self.errors.push(AssemblerError::SymbolAlreadyDeclared);
            return;
        }

        let symbol = Symbol::new(name, SymbolType::Label);
        self.symbols.add_symbol(symbol);
    }

    /// Runs the second pass of the assembler
    fn process_second_phase(&mut self, p: &Program) -> Vec<u8> {
        self.current_instruction = 0;

        let mut program = vec![];

        for i in &p.instructions {
            if i.is_opcode() {
                let mut bytes = i.to_bytes(&self.symbols);
                program.append(&mut bytes);
            }

            if i.is_directive() {
                self.process_directive(i);
            }

            self.current_instruction += 1;
        }

        program
    }
    
    /// Handles a declaration of a section header, such as: .code
    fn process_section_header(&mut self, header_name: &str) -> {
        let new_section: AssemblerSection = header_name.into();

        if new_section == AssemblerSection::Unknown {
            println!("Found an section header that is unknown: {:#?}", header_name);
            return;
        }

        self.sections.push(new_section.clone());
        self.current_section = Some(new_section);
    }

    /// Handles a declaration of a null-terminated string: hello: .asciiz 'Hello!'
    fn handle_asciiz(&mut self, i: &AssemblerInstruction) {
        if self.phase != AssemblerPhase::First { return; }

        match i.get_string_constant() {
            Some(s) => {
                match i.get_label_name() {
                    Some(name) => { self.symbols.set_symbol_offset(&name, self.ro_offset); }
                    None => {
                        println!("Found a string constant with no associated label!");
                        return;
                    }
                };

                for byte in s.as_bytes() {
                    self.ro.push(*byte);
                    self.ro_offset += 1;
                }

                self.ro.push(0);
                self.ro_offset += 1;
            }
            None => {
                println!("String constant following an .asciiz was empty");
            }
        }
    }

    fn extract_labels(&mut self, p: &Program) {
        let mut c = 0;

        for i in &p.instructions {
            if i.is_label() {
                match i.label_name() {
                    Some(name) => {
                        let symbol = Symbol::new(name, SymbolType::Label, c);
                        self.symbols.add_symbol(symbol);
                    }
                    None => {}
                };
            }
            c += 4;
        }
    }

    fn write_pie_header(&self) -> Vec<u8> {
        let mut header = vec![];

        for byte in PIE_HEADER_PREFIX.into_iter() {
            header.push(byte.clone());
        }

        while header.len() <= PIE_HEADER_LENGTH {
            header.push(0 as u8);
        }

        header
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssemblerPhase {
    First,
    Second,
}

impl Default for AssemblerPhase {
    fn default() -> Self {
        AssemblerPhase::First
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssemblerSection {
    Data { starting_instruction: Option<u32> },
    Code { starting_instruction: Option<u32> },
    Unknown,
}

impl Default for AssemblerSection {
    fn default() -> Self {
        AssemblerSection::Unknown
    }
}

impl<'a> From<&'a str> for AssemblerSection {
    fn from(name: &str) -> AssemblerSection {
        match name {
            "data" => AssemblerSection::Data { starting_instruction: None },
            "code" => AssemblerSection::Code { starting_instruction: None },
            _ => AssemblerSection::Unknown,
        }
    }
}

mod tests {
    #![allow(unused_imports)]
    use super::*;
    use crate::vm::VM;

    #[test]
    fn test_symbol_table() {
        let mut sym = SymbolTable::new();
        let new_symbol = Symbol::new("test".to_string(), SymbolType::Label, 12);
        sym.add_symbol(new_symbol);
        assert_eq!(sym.symbols.len(), 1);
        let v = sym.symbol_value("test");
        assert_eq!(true, v.is_some());
        let v = v.unwrap();
        assert_eq!(v, 12);
        let v = sym.symbol_value("does_not_exist");
        assert_eq!(v.is_some(), false);
    }

    #[test]
    fn test_assemble_program() {
        let mut asm = Assembler::new();
        let test_string = "load $0 #100\nload $1 #1\nload $2 #0\ntest: inc $0\nneq $0 $2\njmpe @test\nhlt";
        let program = asm.assemble(test_string).unwrap();
        let mut vm = VM::new();
        assert_eq!(program.len(), 21);
        vm.add_bytes(program);
        assert_eq!(vm.program.len(), 21);
    }
}