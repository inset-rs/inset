//! NumberBoxParser.cpp: the arithmetic a NumberBox with `AcceptsExpression` evaluates. An
//! expression is numbers read by the box's `NumberFormatter`, the operators `+ - * / ^` and
//! parentheses, separated by spaces or nothing. It is turned to postfix order by precedence,
//! `^` over `*` and `/` over `+` and `-`, all binding to the left, and computed.

use super::NumberFormatter;

/// `c_numberBoxOperators`.
const OPERATORS: &str = "+-*/^";

/// `MathToken`: a number, an operator or a parenthesis.
#[derive(Clone, Copy, Debug, PartialEq)]
enum MathToken {
    Numeric(f64),
    Operator(char),
    Parenthesis(char),
}

/// `NumberBoxParser::Compute`: the value of `expression`, or `None` when it is not one. A
/// division by zero is NaN, which is a value.
pub(super) fn compute(expression: &str, parser: &dyn NumberFormatter) -> Option<f64> {
    let tokens = tokens(expression, parser);
    if tokens.is_empty() {
        return None;
    }
    let postfix = infix_to_postfix(&tokens);
    if postfix.is_empty() {
        return None;
    }
    compute_postfix(&postfix)
}

/// `GetTokens`: the tokens of `input`, or none when a part of it is not what its place allows:
/// a number or `(` where a number is expected, an operator or `)` after one. Only spaces are
/// skipped.
fn tokens(input: &str, parser: &dyn NumberFormatter) -> Vec<MathToken> {
    let mut tokens = Vec::new();
    let mut expect_number = true;
    let mut index = 0;
    while let Some(next) = input[index..].chars().next() {
        let mut length = next.len_utf8();
        if next != ' ' {
            if expect_number {
                if next == '(' {
                    // An opening parenthesis keeps a number expected.
                    tokens.push(MathToken::Parenthesis(next));
                } else if let Some((value, number_length)) = next_number(&input[index..], parser) {
                    tokens.push(MathToken::Numeric(value));
                    length = number_length;
                    expect_number = false;
                } else {
                    return Vec::new();
                }
            } else if OPERATORS.contains(next) {
                tokens.push(MathToken::Operator(next));
                expect_number = true;
            } else if next == ')' {
                // A closing parenthesis keeps an operator expected.
                tokens.push(MathToken::Parenthesis(next));
            } else {
                return Vec::new();
            }
        }
        index += length;
    }
    tokens
}

/// `GetNextNumber`: the number at the start of `input` and its length in bytes. What may be a
/// number is an optional `-` and then everything up to an operator, a parenthesis or
/// whitespace (`^-?([^-+/*\(\)\^\s]+)`); `parser` decides whether it is one.
fn next_number(input: &str, parser: &dyn NumberFormatter) -> Option<(f64, usize)> {
    let sign = usize::from(input.starts_with('-'));
    let body = input[sign..]
        .find(|c: char| "-+/*()^".contains(c) || c.is_whitespace())
        .unwrap_or(input.len() - sign);
    if body == 0 {
        return None;
    }
    let length = sign + body;
    parser
        .parse_double(&input[..length])
        .map(|value| (value, length))
}

/// `GetPrecedenceValue`.
fn precedence(operator: char) -> u8 {
    match operator {
        '*' | '/' => 1,
        '^' => 2,
        _ => 0,
    }
}

/// `ConvertInfixToPostfix`: `infix` in postfix order, or none when its parentheses do not pair.
fn infix_to_postfix(infix: &[MathToken]) -> Vec<MathToken> {
    let mut postfix = Vec::new();
    let mut operators: Vec<MathToken> = Vec::new();
    for &token in infix {
        match token {
            MathToken::Numeric(_) => postfix.push(token),
            MathToken::Operator(operator) => {
                while let Some(&top) = operators.last() {
                    match top {
                        MathToken::Operator(previous)
                            if precedence(previous) >= precedence(operator) =>
                        {
                            postfix.push(top);
                            operators.pop();
                        }
                        _ => break,
                    }
                }
                operators.push(token);
            }
            MathToken::Parenthesis('(') => operators.push(token),
            MathToken::Parenthesis(_) => {
                // Operators up to the opening parenthesis go to the output; the pair is dropped.
                loop {
                    match operators.pop() {
                        Some(MathToken::Parenthesis('(')) => break,
                        Some(operator) => postfix.push(operator),
                        None => return Vec::new(),
                    }
                }
            }
        }
    }
    while let Some(top) = operators.pop() {
        if matches!(top, MathToken::Parenthesis(_)) {
            return Vec::new();
        }
        postfix.push(top);
    }
    postfix
}

/// `ComputePostfixExpression`: the value of `tokens`, or `None` when an operator lacks an
/// operand or numbers are left over.
fn compute_postfix(tokens: &[MathToken]) -> Option<f64> {
    let mut stack: Vec<f64> = Vec::new();
    for &token in tokens {
        match token {
            MathToken::Operator(operator) => {
                let right = stack.pop()?;
                let left = stack.pop()?;
                let result = match operator {
                    '-' => left - right,
                    '+' => right + left,
                    '*' => right * left,
                    '/' if right == 0.0 => return Some(f64::NAN),
                    '/' => left / right,
                    '^' => left.powf(right),
                    _ => return None,
                };
                stack.push(result);
            }
            MathToken::Numeric(value) => stack.push(value),
            MathToken::Parenthesis(_) => {}
        }
    }
    match stack.as_slice() {
        [value] => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DecimalFormatter;

    fn evaluate(expression: &str) -> Option<f64> {
        compute(expression, &DecimalFormatter::default())
    }

    #[test]
    fn precedence_puts_powers_over_products_over_sums() {
        assert_eq!(evaluate("1 + 2^2"), Some(5.0));
        assert_eq!(evaluate("2+3*4"), Some(14.0));
        assert_eq!(evaluate("(2+3)*4"), Some(20.0));
        assert_eq!(evaluate("10 - 4 - 3"), Some(3.0));
        assert_eq!(evaluate("2^3^2"), Some(64.0));
        assert_eq!(evaluate("7"), Some(7.0));
    }

    #[test]
    fn a_minus_before_a_number_is_its_sign() {
        assert_eq!(evaluate("-5"), Some(-5.0));
        assert_eq!(evaluate("1--2"), Some(3.0));
        assert_eq!(evaluate("1 - -2"), Some(3.0));
        assert_eq!(evaluate("-(1)"), None);
        assert_eq!(evaluate("- 1"), None);
    }

    #[test]
    fn division_by_zero_is_nan() {
        assert!(evaluate("1/0").is_some_and(f64::is_nan));
        assert_eq!(evaluate("1/4"), Some(0.25));
    }

    #[test]
    fn malformed_expressions_have_no_value() {
        for expression in [
            "", "   ", "(", "()", "(1+2", "1+2)", "1+", "+1", "2(3)", "1 2", "abc", "1,5", "1\t+2",
        ] {
            assert_eq!(evaluate(expression), None, "{expression:?}");
        }
    }
}
