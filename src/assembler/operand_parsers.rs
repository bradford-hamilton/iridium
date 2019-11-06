use nom::digit;
use nom::types::CompleteStr;

use crate::assembler::Token;

/// Parser for integer numbers, which we preface with `#` in our assembly language:
/// #100
named!(integer_operand<CompleteStr, Token>,
    ws!(
        do_parse!(
            tag!("#") >>
            reg_num: digit >>
            (
                Token::IntegerOperand{ value: reg_num.parse::<i32>().unwrap() }
            )
        )
    )
);

mod tests {
    use super::*;

    #[test]
    fn test_parse_integer_operand() {
        // Test a valid integer operand
        let result = integer_operand(CompleteStr("#10"));
        assert_eq!(result.is_ok(), true);
        let (rest, value) = result.unwrap();
        assert_eq!(rest, CompleteStr(""));
        assert_eq!(value, Token::IntegerOperand { value: 10 });

        // Test an invalid one (missing the #)
        let result = integer_operand(CompleteStr("10"));
        assert_eq!(result.is_ok(), false);
    }
}
