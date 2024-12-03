use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_while},
    character::complete::{crlf, digit1, one_of},
    combinator::{map_res, opt, value},
    error::ErrorKind,
    multi::many_m_n,
    number::complete::double,
    sequence::{delimited, pair, preceded},
    Err, IResult,
};

use super::Message;

fn parse_simple_string(i: &[u8]) -> IResult<&[u8], Message> {
    let (remaining, parsed) = delimited(
        tag("+"),
        take_while(|b| b != b'\r' && b != b'\n'),
        tag("\r\n"),
    )(i)?;
    Ok((remaining, Message::SimpleString(parsed)))
}

fn parse_error(i: &[u8]) -> IResult<&[u8], Message> {
    let (remaining, parsed) = delimited(
        tag("-"),
        take_while(|b| b != b'\r' && b != b'\n'),
        tag("\r\n"),
    )(i)?;
    Ok((remaining, Message::Error(parsed)))
}

fn parse_signed_integer(i: &[u8]) -> IResult<&[u8], isize> {
    map_res(
        pair(opt(one_of("+-")), digit1),
        |(sign, digits): (Option<char>, &[u8])| {
            let s = String::from_utf8_lossy(digits);
            s.parse::<isize>()
                .map(|mut n| {
                    if sign == Some('-') {
                        n = -n;
                    }
                    n
                })
                .map_err(|_| nom::Err::Error((digits, ErrorKind::Digit)))
        },
    )(i)
}

fn parse_integer(i: &[u8]) -> IResult<&[u8], Message> {
    map_res(
        delimited(tag(":"), parse_signed_integer, tag("\r\n")),
        // Cursed type hint here because return type cannot be inferred
        |n: isize| Ok::<Message, Err<nom::error::Error<&[u8]>>>(Message::Integer(n)),
    )(i)
}

fn parse_bulk_string(i: &[u8]) -> IResult<&[u8], Message> {
    let (remaining, length) = preceded(tag("$"), parse_signed_integer)(i)?;

    if length == -1 {
        let (remaining, _) = crlf(remaining)?;
        return Ok((remaining, Message::BulkString(None)));
    }

    let (remaining, parsed) = delimited(crlf, take(length as usize), crlf)(remaining)?;
    Ok((remaining, Message::BulkString(Some(parsed))))
}

fn parse_array(i: &[u8]) -> IResult<&[u8], Message> {
    let (remaining, length) = delimited(tag("*"), parse_signed_integer, crlf)(i)?;

    if length == -1 {
        let (remaining, _) = crlf(remaining)?;
        return Ok((remaining, Message::Array(None)));
    }

    let length = length as usize;
    let (remaining, parsed) = many_m_n(length, length, parse_message)(remaining)?;
    Ok((remaining, Message::Array(Some(parsed))))
}

fn parse_null(i: &[u8]) -> IResult<&[u8], Message> {
    let (remaining, _) = pair(tag("_"), crlf)(i)?;
    Ok((remaining, Message::Null))
}

fn parse_bool(i: &[u8]) -> IResult<&[u8], Message> {
    let (remaining, result) = delimited(
        tag("#"),
        alt((value(true, tag("t")), value(false, tag("f")))),
        crlf,
    )(i)?;
    Ok((remaining, Message::Bool(result)))
}

fn parse_double(i: &[u8]) -> IResult<&[u8], Message> {
    let (remaining, result) = delimited(tag(","), double, crlf)(i)?;
    Ok((remaining, Message::Double(result)))
}

// Main export
pub(crate) fn parse_message(i: &[u8]) -> IResult<&[u8], Message> {
    alt((
        parse_simple_string,
        parse_error,
        parse_integer,
        parse_bulk_string,
        parse_array,
        parse_null,
        parse_bool,
        parse_double,
    ))(i)
}

#[cfg(test)]
mod test {
    use crate::messages::*;

    fn parse_double_helper(input: &[u8]) -> IResult<&[u8], Message> {
        parse_double(input)
    }

    #[test]
    fn test_parse_double() {
        // Valid double with no sign, no exponent
        let input = b",123.456\r\n";
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => {
                assert_eq!(parsed, Message::Double(123.456));
                assert_eq!(remaining, &[]); // No remaining input
            }
            Err(e) => panic!("Failed to parse valid double: {:?}", e),
        }

        // Valid double with positive sign
        let input = b",+123.456\r\n";
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => {
                assert_eq!(parsed, Message::Double(123.456));
                assert_eq!(remaining, &[]);
            }
            Err(e) => panic!("Failed to parse double with positive sign: {:?}", e),
        }

        // Valid double with negative sign
        let input = b",-123.456\r\n";
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => {
                assert_eq!(parsed, Message::Double(-123.456));
                assert_eq!(remaining, &[]);
            }
            Err(e) => panic!("Failed to parse double with negative sign: {:?}", e),
        }

        // Valid double with exponent (positive)
        let input = b",123.456e+7\r\n";
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => {
                assert_eq!(parsed, Message::Double(123.456e+7));
                assert_eq!(remaining, &[]);
            }
            Err(e) => panic!("Failed to parse double with exponent: {:?}", e),
        }

        // Valid double with exponent (negative)
        let input = b",123.456e-7\r\n";
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => {
                assert_eq!(parsed, Message::Double(123.456e-7));
                assert_eq!(remaining, &[]);
            }
            Err(e) => panic!("Failed to parse double with negative exponent: {:?}", e),
        }

        // Valid double with fractional part but no exponent
        let input = b",123.456\r\n";
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => {
                assert_eq!(parsed, Message::Double(123.456));
                assert_eq!(remaining, &[]);
            }
            Err(e) => panic!("Failed to parse double with fractional part: {:?}", e),
        }

        // Valid double with just integral part
        let input = b",123\r\n";
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => {
                assert_eq!(parsed, Message::Double(123.0));
                assert_eq!(remaining, &[]);
            }
            Err(e) => panic!("Failed to parse integer as double: {:?}", e),
        }

        // Invalid double, non-numeric
        let input = b",abc\r\n";
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => panic!("Expected error, but parsed: {:?}", parsed),
            Err(e) => {
                println!("Expected error: {:?}", e);
                assert!(true); // Test passes because error was expected
            }
        }

        // Invalid double, missing CRLF terminator
        let input = b",123.456";
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => panic!("Expected error, but parsed: {:?}", parsed),
            Err(e) => {
                println!("Expected error: {:?}", e);
                assert!(true);
            }
        }

        // Invalid double, missing comma
        let input = b"123.456\r\n"; // Missing leading comma
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => panic!("Expected error, but parsed: {:?}", parsed),
            Err(e) => {
                println!("Expected error: {:?}", e);
                assert!(true);
            }
        }

        // Valid double with scientific notation (upper case 'E')
        let input = b",1.23E+4\r\n";
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => {
                assert_eq!(parsed, Message::Double(1.23E+4));
                assert_eq!(remaining, &[]);
            }
            Err(e) => panic!("Failed to parse double with uppercase 'E': {:?}", e),
        }

        // Valid double with scientific notation (lower case 'e')
        let input = b",1.23e+4\r\n";
        let result = parse_double_helper(input);
        match result {
            Ok((remaining, parsed)) => {
                assert_eq!(parsed, Message::Double(1.23e+4));
                assert_eq!(remaining, &[]);
            }
            Err(e) => panic!("Failed to parse double with lowercase 'e': {:?}", e),
        }
    }

    #[test]
    fn test_parse_array_with_single_strings() {
        let input = b"*1\r\n+hello\r\n"; // An array with a simple string, integer, and bulk string
        let result = parse_array_helper(input);

        match result {
            Ok((remaining, parsed)) => {
                println!("Parsed: {:?}", parsed); // Print the parsed result
                assert_eq!(
                    parsed,
                    Message::Array(Some(vec![Message::SimpleString("hello".to_string()),]))
                );
            }
            Err(e) => {
                print_error(input, e); // Print error details
                panic!("Failed to parse array");
            }
        }
    }

    #[test]
    fn test_simple_string() {
        let test_string = "+some string\r\n";
        assert_eq!(
            Ok((
                "".as_bytes(),
                Message::SimpleString("some string".to_string())
            )),
            parse_simple_string(test_string.as_bytes())
        );

        let test_string = "-some error\r\n";
        assert!(parse_simple_string(test_string.as_bytes()).is_err());

        let test_string = "bad\r\n";
        assert!(parse_simple_string(test_string.as_bytes()).is_err());

        let test_string = "+bad";
        assert!(parse_simple_string(test_string.as_bytes()).is_err());
    }

    #[test]
    fn test_error() {
        let test_string = "-some string\r\n";
        assert_eq!(
            Ok(("".as_bytes(), Message::Error("some string".to_string()))),
            parse_error(test_string.as_bytes())
        );

        let test_string = "+some error\r\n";
        assert!(parse_error(test_string.as_bytes()).is_err());

        let test_string = "bad\r\n";
        assert!(parse_error(test_string.as_bytes()).is_err());

        let test_string = "+bad";
        assert!(parse_error(test_string.as_bytes()).is_err());
    }

    #[test]
    fn test_parse_integer() {
        // Test valid positive integer
        let input = b":123\r\n";
        let result = parse_integer(input);
        assert_eq!(result, Ok((&[][..], Message::Integer(123))));

        // Test valid negative integer
        let input = b":-123\r\n";
        let result = parse_integer(input);
        assert_eq!(result, Ok((&[][..], Message::Integer(-123))));

        // Test integer with no sign
        let input = b":456\r\n";
        let result = parse_integer(input);
        assert_eq!(result, Ok((&[][..], Message::Integer(456))));

        // Test invalid integer (non-numeric characters)
        let input = b":abc\r\n";
        let result = parse_integer(input);
        assert!(result.is_err());

        // Test invalid format (missing CRLF)
        let input = b":123\r";
        let result = parse_integer(input);
        assert!(result.is_err());

        // Test invalid integer (empty)
        let input = b":\r\n";
        let result = parse_integer(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_bulk_string() {
        // Test valid bulk string with data
        let input = b"$5\r\nhello\r\n";
        let result = parse_bulk_string(input);
        assert_eq!(
            result,
            Ok((&[][..], Message::BulkString(Some("hello".to_string()))))
        );

        // Test valid bulk string with zero length
        let input = b"$0\r\n\r\n";
        let result = parse_bulk_string(input);
        assert_eq!(
            result,
            Ok((&[][..], Message::BulkString(Some("".to_string()))))
        );

        // Test invalid bulk string (non-digit length)
        let input = b"$abc\r\nhello\r\n";
        let result = parse_bulk_string(input);
        assert!(result.is_err());

        // Test bulk string with invalid length (too short)
        let input = b"$5\r\nhell\r\n";
        let result = parse_bulk_string(input);
        assert!(result.is_err());

        // Test bulk string with missing CRLF terminator
        let input = b"$5\r\nhello";
        let result = parse_bulk_string(input);
        assert!(result.is_err());
    }

    // Helper function to test parsing of arrays
    fn parse_array_helper(input: &[u8]) -> IResult<&[u8], Message> {
        parse_array(input)
    }

    // Helper function to print errors in a human-readable ASCII format
    fn print_error(input: &[u8], error: nom::Err<nom::error::Error<&[u8]>>) {
        // Convert the input bytes to a human-readable string (ASCII)
        let readable_input = String::from_utf8_lossy(input);
        println!(
            "Parsing Error: Error {{ input: {:?}, code: {:?} }}",
            readable_input, error
        );
    }

    #[test]
    fn test_parse_array_with_simple_strings() {
        let input = b"*3\r\n+hello\r\n:123\r\n$5\r\nworld\r\n"; // An array with a simple string, integer, and bulk string
        let result = parse_array_helper(input);

        match result {
            Ok((remaining, parsed)) => {
                println!("Parsed: {:?}", parsed); // Print the parsed result
                assert_eq!(
                    parsed,
                    Message::Array(Some(vec![
                        Message::SimpleString("hello".to_string()),
                        Message::Integer(123),
                        Message::BulkString(Some("world".to_string())),
                    ]))
                );
            }
            Err(e) => {
                print_error(input, e); // Print error details
                panic!("Failed to parse array");
            }
        }
    }

    #[test]
    fn test_parse_empty_array() {
        let input = b"*0\r\n"; // An empty array
        let result = parse_array_helper(input);

        match result {
            Ok((remaining, parsed)) => {
                println!("Parsed: {:?}", parsed);
                assert_eq!(parsed, Message::Array(Some(vec![])));
            }
            Err(e) => {
                print_error(input, e); // Print error details
                panic!("Failed to parse empty array");
            }
        }
    }

    #[test]
    fn test_parse_array_with_null() {
        let input = b"*1\r\n$-1\r\n"; // An array with a single NULL element
        let result = parse_array_helper(input);

        match result {
            Ok((remaining, parsed)) => {
                println!("Parsed: {:?}", parsed);
                assert_eq!(
                    parsed,
                    Message::Array(Some(vec![Message::BulkString(None)]))
                );
            }
            Err(e) => {
                print_error(input, e); // Print error details
                panic!("Failed to parse null array");
            }
        }
    }

    #[test]
    fn test_parse_invalid_array_length() {
        let input = b"*abc\r\n"; // Invalid array length (non-numeric)
        let result = parse_array_helper(input);

        match result {
            Ok((remaining, parsed)) => {
                println!("Parsed: {:?}", parsed);
                panic!("Expected error, but parsed: {:?}", parsed);
            }
            Err(e) => {
                print_error(input, e); // Print error details
                assert!(true); // Test passes because error was expected
            }
        }
    }

    #[test]
    fn test_parse_invalid_array_format() {
        let input = b"*3\r\n+hello\r\n:123\r\n"; // Invalid array format (missing CRLF after $5)
        let result = parse_array_helper(input);

        match result {
            Ok((remaining, parsed)) => {
                println!("Parsed: {:?}", parsed);
                panic!("Expected error, but parsed: {:?}", parsed);
            }
            Err(e) => {
                print_error(input, e); // Print error details
                assert!(true); // Test passes because error was expected
            }
        }
    }

    #[test]
    fn test_parse_array_with_mixed_messages() {
        let input = b"*4\r\n+simple\r\n$5\r\nbulk1\r\n:456\r\n-Error\r\n"; // Mixed message types in the array
        let result = parse_array_helper(input);

        match result {
            Ok((remaining, parsed)) => {
                println!("Parsed: {:?}", parsed); // Print parsed result
                assert_eq!(
                    parsed,
                    Message::Array(Some(vec![
                        Message::SimpleString("simple".to_string()),
                        Message::BulkString(Some("bulk1".to_string())),
                        Message::Integer(456),
                        Message::Error("Error".to_string())
                    ]))
                );
            }
            Err(e) => {
                print_error(input, e); // Print error details
                panic!("Failed to parse mixed array");
            }
        }
    }

    #[test]
    fn test_parse_array_with_mixed_elements() {
        let input = b"*3\r\n$5\r\nhello\r\n$-1\r\n$5\r\nworld\r\n"; // Input representing the array with bulk strings

        let result = parse_array_helper(input);

        match result {
            Ok((remaining, parsed)) => {
                println!("Parsed: {:?}", parsed); // Print the parsed result

                // Assert that the result is the expected array with the three elements
                assert_eq!(
                    parsed,
                    Message::Array(Some(vec![
                        Message::BulkString(Some("hello".to_string())),
                        Message::BulkString(None), // NULL element
                        Message::BulkString(Some("world".to_string())),
                    ]))
                );
            }
            Err(e) => {
                println!("Parsing Error: {:?}", e); // Print error details if parsing fails
                panic!("Failed to parse array");
            }
        }
    }
}
