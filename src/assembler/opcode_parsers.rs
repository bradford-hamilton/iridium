use nom::types::CompleteStr;
use nom::*;

use crate::assembler::Token;
use crate::instruction::Opcode;

named!(pub opcode<CompleteStr, Token>,
  do_parse!(
      opcode: alpha1 >>
      (
        {
            Token::Op { code: Opcode::from(opcode) }
        }
      )
  )
);

mod tests {
    #![allow(unused_imports)]
    use super::*;

    #[test]
    fn test_opcode_load() {
        // First tests that the opcode is detected and parsed correctly
        let result = opcode(CompleteStr("load"));
        assert_eq!(result.is_ok(), true);
        let (rest, token) = result.unwrap();
        assert_eq!(token, Token::Op { code: Opcode::LOAD });
        assert_eq!(rest, CompleteStr(""));

        // Tests that an invalid opcode isn't recognized
        let result = opcode(CompleteStr("aold"));
        assert_eq!(result.is_ok(), false);
    }
}
