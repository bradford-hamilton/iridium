use crate::assembler::label_parsers::label_declaration;

named!(directive_declaration<CompleteStr, Token>,
  do_parse!(
      tag!(".") >>
      name: alpha1 >>
      (
        Token::Directive{name: name.to_string()}
      )
  )
);

named!(directive_combined<CompleteStr, AssemblerInstruction>,
    ws!(
        do_parse!(
            l: opt!(label_declaration) >>
            name: directive_declaration >>
            o1: opt!(operand) >>
            o2: opt!(operand) >>
            o3: opt!(operand) >>
            (
                AssemblerInstruction{
                    opcode: None,
                    directive: Some(name),
                    label: l,
                    operand1: o1,
                    operand2: o2,
                    operand3: o3,
                }
            )
        )
    )
);

/// Will try to parse out any of the Directive forms
named!(pub directive<CompleteStr, AssemblerInstruction>,
    do_parse!(
        ins: alt!(
            directive_combined
        ) >>
        (
            ins
        )
    )
);


mod tests {
    #![allow(unused_imports)]
    use super::*;

    #[test]
    fn test_string_directive() {
        let result = directive_combined(CompleteStr("test: .asciiz 'Hello'"));
        assert_eq!(result.is_ok(), true);
        let (_, directive) = result.unwrap();

        // Yes, this is the what the result should be
        let correct_instruction =
            AssemblerInstruction {
                opcode: None,
                label: Some(
                    Token::LabelDeclaration {
                        name: "test".to_string()
                    }),
                directive: Some(d
                    Token::Directive {
                        name: "asciiz".to_string()
                    }),
                operand1: Some(Token::IrString { name: "Hello".to_string() }),
                operand2: None,
                operand3: None };

        assert_eq!(directive, correct_instruction);
    }
}