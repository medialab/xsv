// En tant que chef, je m'engage à ce que nous ne nous fassions pas *tous* tuer.
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, anychar, char, digit1, none_of, space0},
    combinator::{all_consuming, map_res, not, opt, recognize, value},
    multi::{fold_many0, many0, separated_list0},
    number::complete::double,
    sequence::{delimited, pair, terminated, tuple},
    IResult,
};

#[derive(Debug, PartialEq)]
enum Argument {
    Identifier(String),
    StringLiteral(String),
    FloatLiteral(f64),
    IntegerLiteral(i64),
    BooleanLiteral(bool),
    Underscore,
}

#[derive(Debug, PartialEq)]
struct FunctionCall {
    name: String,
    args: Vec<Argument>,
}

fn boolean_literal(input: &str) -> IResult<&str, bool> {
    alt((value(true, tag("true")), value(false, tag("false"))))(input)
}

fn float_literal(input: &str) -> IResult<&str, f64> {
    double(input)
}

fn underscore(input: &str) -> IResult<&str, ()> {
    value((), char('_'))(input)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alpha1,
        many0(alt((alphanumeric1, tag("_"), tag("-")))),
    ))(input)
}

fn integer_literal(input: &str) -> IResult<&str, i64> {
    map_res(
        recognize(pair(digit1, many0(alt((digit1, tag("_")))))),
        |string: &str| string.replace("_", "").parse::<i64>(),
    )(input)
}

fn string_character_literal(input: &str) -> IResult<&str, char> {
    let (input, c) = none_of("\"")(input)?;

    if c == '\\' {
        let (input, c) = anychar(input)?;

        Ok((
            input,
            match c {
                '"' | '\\' | '/' => c,
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                _ => {
                    return Err(nom::Err::Failure(nom::error::ParseError::from_char(
                        input, c,
                    )))
                }
            },
        ))
    } else {
        Ok((input, c))
    }
}

fn string_literal(input: &str) -> IResult<&str, String> {
    delimited(
        char('"'),
        fold_many0(string_character_literal, String::new, |mut string, c| {
            string.push(c);
            string
        }),
        char('"'),
    )(input)
}

fn argument_separator(input: &str) -> IResult<&str, ()> {
    value((), tuple((space0, char(','), space0)))(input)
}

fn argument(input: &str) -> IResult<&str, Argument> {
    alt((
        map_res(boolean_literal, |value| -> Result<Argument, ()> {
            Ok(Argument::BooleanLiteral(value))
        }),
        map_res(identifier, |name| -> Result<Argument, ()> {
            Ok(Argument::Identifier(String::from(name)))
        }),
        map_res(
            terminated(integer_literal, not(char('.'))),
            |value| -> Result<Argument, ()> { Ok(Argument::IntegerLiteral(value)) },
        ),
        map_res(float_literal, |value| -> Result<Argument, ()> {
            Ok(Argument::FloatLiteral(value))
        }),
        map_res(underscore, |_| -> Result<Argument, ()> {
            Ok(Argument::Underscore)
        }),
    ))(input)
}

fn argument_list(input: &str) -> IResult<&str, Vec<Argument>> {
    separated_list0(argument_separator, argument)(input)
}

fn function_call(input: &str) -> IResult<&str, FunctionCall> {
    map_res(
        pair(
            identifier,
            opt(delimited(
                pair(space0, char('(')),
                argument_list,
                pair(char(')'), space0),
            )),
        ),
        |(name, args)| -> Result<FunctionCall, ()> {
            Ok(FunctionCall {
                name: String::from(name),
                args: args.unwrap_or_else(|| vec![Argument::Underscore]),
            })
        },
    )(input)
}

fn pipe(input: &str) -> IResult<&str, ()> {
    value((), tuple((space0, char('|'), space0)))(input)
}

fn pipeline(input: &str) -> IResult<&str, Vec<FunctionCall>> {
    all_consuming(separated_list0(pipe, function_call))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boolean_literal() {
        assert_eq!(boolean_literal("true, test"), Ok((", test", true)));

        assert_eq!(boolean_literal("false"), Ok(("", false)));
    }

    #[test]
    fn test_float_literal() {
        assert_eq!(float_literal("3.56"), Ok(("", 3.56f64)))
    }

    #[test]
    fn test_integer_literal() {
        assert_eq!(integer_literal("456_400"), Ok(("", 456_400i64)));
    }

    #[test]
    fn test_string_literal() {
        assert_eq!(
            string_literal(r#""hello", 45"#),
            Ok((", 45", String::from("hello")))
        );
        assert_eq!(
            string_literal(r#""héllo", 45"#),
            Ok((", 45", String::from("héllo")))
        );
        assert_eq!(
            string_literal(r#""hel\nlo", 45"#),
            Ok((", 45", String::from("hel\nlo")))
        );
    }

    #[test]
    fn test_underscore() {
        assert_eq!(underscore("_, 45"), Ok((", 45", ())))
    }

    #[test]
    fn test_identifier() {
        assert_eq!(identifier("input, test"), Ok((", test", "input")));
    }

    #[test]
    fn test_argument() {
        assert_eq!(argument("true"), Ok(("", Argument::BooleanLiteral(true))));
    }

    #[test]
    fn test_argument_list() {
        assert_eq!(argument_list(""), Ok(("", vec![])));
        assert_eq!(
            argument_list("true, _, col0"),
            Ok((
                "",
                vec![
                    Argument::BooleanLiteral(true),
                    Argument::Underscore,
                    Argument::Identifier(String::from("col0"))
                ]
            ))
        )
    }

    #[test]
    fn test_function_call() {
        assert_eq!(
            function_call("trim()"),
            Ok((
                "",
                FunctionCall {
                    name: String::from("trim"),
                    args: vec![]
                }
            ))
        );

        assert_eq!(
            function_call("trim(_)"),
            Ok((
                "",
                FunctionCall {
                    name: String::from("trim"),
                    args: vec![Argument::Underscore]
                }
            ))
        );

        assert_eq!(
            function_call("trim(_, true, 4.5, 56, col)"),
            Ok((
                "",
                FunctionCall {
                    name: String::from("trim"),
                    args: vec![
                        Argument::Underscore,
                        Argument::BooleanLiteral(true),
                        Argument::FloatLiteral(4.5),
                        Argument::IntegerLiteral(56),
                        Argument::Identifier(String::from("col"))
                    ]
                }
            ))
        );
    }

    #[test]
    fn test_pipeline() {
        assert!(pipeline("test |").is_err());

        assert_eq!(
            pipeline("trim(name) | len  (_)"),
            Ok((
                "",
                vec![
                    FunctionCall {
                        name: String::from("trim"),
                        args: vec![Argument::Identifier(String::from("name"))]
                    },
                    FunctionCall {
                        name: String::from("len"),
                        args: vec![Argument::Underscore]
                    }
                ]
            ))
        );

        assert_eq!(
            pipeline("trim(name)|len  (_)"),
            Ok((
                "",
                vec![
                    FunctionCall {
                        name: String::from("trim"),
                        args: vec![Argument::Identifier(String::from("name"))]
                    },
                    FunctionCall {
                        name: String::from("len"),
                        args: vec![Argument::Underscore]
                    }
                ]
            ))
        );

        assert_eq!(
            pipeline("trim(name) | len(_)  "),
            Ok((
                "",
                vec![
                    FunctionCall {
                        name: String::from("trim"),
                        args: vec![Argument::Identifier(String::from("name"))]
                    },
                    FunctionCall {
                        name: String::from("len"),
                        args: vec![Argument::Underscore]
                    }
                ]
            ))
        );

        assert_eq!(
            pipeline("trim | len"),
            Ok((
                "",
                vec![
                    FunctionCall {
                        name: String::from("trim"),
                        args: vec![Argument::Underscore]
                    },
                    FunctionCall {
                        name: String::from("len"),
                        args: vec![Argument::Underscore]
                    }
                ]
            ))
        );
    }
}
