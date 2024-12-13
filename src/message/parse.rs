use core::str;

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

const CRLF: &[u8] = b"\r\n";

macro_rules! check_tag {
    ($target:expr, $input:expr) => {{
        // Safely check and consume the first byte of the input
        if let Some(&tag) = ($input).get(0) {
            assert_eq!(tag, $target, "Expected tag {:?}, but found {:?}", $target, tag);
            $input = &$input[1..]; // Update the input reference
        } else {
            panic!("Input is empty, expected tag {:?}", $target);
        }
    }};
}


fn parse_simple_string(mut i: &[u8]) -> Result<(&[u8], Message), &str> {
    check_tag!(b'+', i);
    if let Some(pos) = i.windows(2).position(|window| window == CRLF) {
        let content = &i[..pos];
        let message = Message::SimpleString(String::from_utf8_lossy(content).to_string());
        let remaining = &i[pos+2..];
        Ok((remaining, message))
    } else {
        Err("simple string parse error")
    }
}

fn parse_error(mut i: &[u8]) -> Result<(&[u8], Message), &str> {
    check_tag!(b'-', i);
    if let Some(pos) = i.windows(2).position(|window| window == CRLF) {
        let content = &i[..pos];
        let message = Message::SimpleString(String::from_utf8_lossy(content).to_string());
        let remaining = &i[pos+2..];
        Ok((remaining, message))
    } else {
        Err("error parse error")
    }
}

fn parse_signed_integer(mut i: &[u8]) -> Result<(&[u8], isize), &str> {
    let maybe_sign = i.get(0).unwrap();
    if b"+-".contains(maybe_sign) {
        i = &i[1..];
    }
    if let Some(pos) = i.windows(1).position(|window| !window[0].is_ascii_digit()) {
        let content = &i[..pos];
        let mut number: isize = String::from_utf8_lossy(content).parse().unwrap();
        if *maybe_sign == b'-' { number = -number; }
        Ok((&i[pos..], number))
    } else {
        Err("parse signed int error")
    }
}

fn parse_integer(mut i: &[u8]) -> Result<(&[u8], Message), &str> {
    check_tag!(b':', i);
    let (i, n) = parse_signed_integer(i).unwrap();
    let message = Message::Integer(n);
    Ok((&i[2..], message))
}

fn parse_bulk_string(mut i: &[u8]) -> Result<(&[u8], Message), &str> {
    check_tag!(b'$', i);
    let (mut i, length) = parse_signed_integer(i).unwrap();

    if length == -1 {
        return Ok((&i[2..], Message::BulkString(None)));
    }

    i = &i[2..]; //CRLF
    let length = length as usize;
    let content = &i[..length];
    let string = String::from_utf8_lossy(content).to_string();
    let message = Message::BulkString(Some(string));
    return Ok((&i[length+2..], message));
}

fn parse_array(mut i: &[u8]) -> Result<(&[u8], Message), &str> {
    check_tag!(b'*', i);
    let (mut i, length) = parse_signed_integer(i).unwrap();

    if length == -1 {
        return Ok((&i[2..], Message::Array(None)));
    }

    i = &i[2..]; // CRLF
    let length = length as usize;
    let mut messages = Vec::new();
    for _ in 0..length {
        let (remaining, message) = parse_message(i).unwrap();
        i = remaining;
        messages.push(message);
    }
    Ok((i, Message::Array(Some(messages))))
}

fn parse_null(mut i: &[u8]) -> Result<(&[u8], Message), &str> {
    check_tag!(b'_', i);
    Ok((&i[2..], Message::Null))
}

fn parse_bool(mut i: &[u8]) -> Result<(&[u8], Message), &str> {
    check_tag!(b'#', i);
    let value = match i.get(0).unwrap() {
        b't' => true,
        b'f' => false,
        _ => panic!(),
    };

    Ok((&i[3..], Message::Bool(value)))
}

fn parse_double(mut i: &[u8]) -> Result<(&[u8], Message), &str> {
    check_tag!(b',', i);
    if let Some(pos) = i.windows(2).position(|window| window == CRLF) {
        let content = &i[..pos];
        let double = str::from_utf8(content).unwrap().parse::<f64>().unwrap();
        Ok((&i[pos+2..], Message::Double(double)))
    } else {
        Err("")
    }
}

// Main export
pub(crate) fn parse_message(i: &[u8]) -> Result<(&[u8], Message), &str> {
    let tag= i.get(0).unwrap();
    let (remaining, message) = match *tag {
        b'+' => parse_simple_string(i),
        b'-' => parse_error(i),
        b':' => parse_integer(i),
        b'$' => parse_bulk_string(i),
        b'*' => parse_array(i),
        b'_' => parse_null(i),
        b'#' => parse_bool(i),
        b',' => parse_double(i),
        _ => panic!(),
    }.unwrap();
    Ok((remaining, message))
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
