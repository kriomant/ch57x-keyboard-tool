///! Collection of NOM parsers for various things.
///! Generally only `parse` and `from_str` functions should be called
///! from outside of this module, they ensures that whole input is
///! consumed.
///! Other functions are composable parsers for use within this module
///! or as parameters for functions mentioned above.

use nom::{
    Parser, IResult, InputLength,
    branch::alt,
    // Removed `pair` as it's not directly used in new code, added `char as nom_char` for clarity
    sequence::{tuple, terminated, separated_pair, delimited},
    multi::{separated_list1, fold_many0},
    bytes::complete::tag,
    character::complete::{char as nom_char, alpha1, alphanumeric1, digit1}, // Renamed char to nom_char
    combinator::{map, map_res, opt, all_consuming, value},
    error::ParseError,
};

// ADDED IMPORTS
use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::keyboard::{Accord, Modifier, Modifiers, Macro, MouseEvent, MouseModifier, MouseButton, MouseButtons, MouseAction, MediaCode, Code, WellKnownCode};

use std::str::FromStr;

fn mouse_modifier(s: &str) -> IResult<&str, MouseModifier> {
    map_res(alpha1, MouseModifier::from_str)(s)
 }

fn media_code(s: &str) -> IResult<&str, MediaCode> {
    map_res(alpha1, MediaCode::from_str)(s)
}

// NEW: ParsedUnit enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedUnit {
    DirectCode(Code),
    ShiftedCode(Code), // The Code here is the base key (e.g., N1 for '!')
    CustomCode(u8),
}

// NEW: SHIFTED_CHARS_MAP
static SHIFTED_CHARS_MAP: Lazy<HashMap<char, WellKnownCode>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert('!', WellKnownCode::N1);
    m.insert('@', WellKnownCode::N2);
    m.insert('#', WellKnownCode::N3);
    m.insert('$', WellKnownCode::N4);
    m.insert('%', WellKnownCode::N5);
    m.insert('^', WellKnownCode::N6);
    m.insert('&', WellKnownCode::N7);
    m.insert('*', WellKnownCode::N8);
    m.insert('(', WellKnownCode::N9);
    m.insert(')', WellKnownCode::N0);
    m.insert('_', WellKnownCode::Minus);
    m.insert('+', WellKnownCode::Equal);
    m.insert('{', WellKnownCode::LeftBracket);
    m.insert('}', WellKnownCode::RightBracket);
    m.insert('|', WellKnownCode::Backslash);
    m.insert(':', WellKnownCode::Semicolon);
    m.insert('"', WellKnownCode::Quote); // char literal for double quote
    m.insert('~', WellKnownCode::Grave);
    m.insert('<', WellKnownCode::Comma); // Note: This is for shifted Comma. CustomCode <XX> is different.
    m.insert('>', WellKnownCode::Dot);   // Note: This is for shifted Dot.
    m.insert('?', WellKnownCode::Slash);
    m
});

// NEW: single_key_unit parser
fn single_key_unit(s: &str) -> IResult<&str, ParsedUnit> {
    if s.is_empty() {
        return Err(nom::Err::Error(nom::error::make_error(s, nom::error::ErrorKind::Eof)));
    }

    // 1. Try custom code like <DD>
    // Must come before single char parsing to correctly parse '<' as part of custom code.
    if let Ok((rest, code_val)) = delimited(
        nom_char::<_, nom::error::Error<&str>>('<'),
        map_res(digit1::<_, nom::error::Error<&str>>, str::parse),
        nom_char::<_, nom::error::Error<&str>>('>')
    )(s) {
        // Check if the char after '>' is a delimiter or end of string, to avoid parsing e.g. "<12>a" as custom code for 'a'
        if rest.is_empty() || !rest.starts_with(|c: char| c.is_alphanumeric() || SHIFTED_CHARS_MAP.contains_key(&c)) {
             return Ok((rest, ParsedUnit::CustomCode(code_val)));
        }
    }

    // 2. Try single character (shifted or direct)
    let first_char = s.chars().next().unwrap(); // Safe due to earlier is_empty check
    let rest_of_s = &s[first_char.len_utf8()..];

    // Check if it's a shifted symbol from our map
    if let Some(base_code) = SHIFTED_CHARS_MAP.get(&first_char) {
        return Ok((rest_of_s, ParsedUnit::ShiftedCode(Code::WellKnown(*base_code))));
    }

    // Check if it's a non-alphanumeric WellKnownCode (e.g., '-', '[', ' ')
    let direct_code_opt: Option<WellKnownCode> = match first_char {
        '-' => Some(WellKnownCode::Minus),
        '=' => Some(WellKnownCode::Equal),
        '[' => Some(WellKnownCode::LeftBracket),
        ']' => Some(WellKnownCode::RightBracket),
        '\\' => Some(WellKnownCode::Backslash), // char for backslash
        ';' => Some(WellKnownCode::Semicolon),
        '\'' => Some(WellKnownCode::Quote),   // char for single quote
        '`' => Some(WellKnownCode::Grave),
        ',' => Some(WellKnownCode::Comma),
        '.' => Some(WellKnownCode::Dot),
        '/' => Some(WellKnownCode::Slash),
        ' ' => Some(WellKnownCode::Space),
        _ => None,
    };

    if let Some(wk_code) = direct_code_opt {
        return Ok((rest_of_s, ParsedUnit::DirectCode(Code::WellKnown(wk_code))));
    }

    // 3. Try alphanumeric WellKnownCode (like 'a', 'f1', 'enter')
    // This must come after checking SHIFTED_CHARS_MAP and direct punctuation
    // to ensure things like "minus" are parsed as `WellKnownCode::Minus`
    // and not single char '-' followed by "inus".
    match map_res(
        alphanumeric1::<_, nom::error::Error<&str>>,
        |name: &str| WellKnownCode::from_str(name)
    )(s) {
        Ok((rest, wk_code)) => {
            Ok((rest, ParsedUnit::DirectCode(Code::WellKnown(wk_code))))
        }
        Err(_e) => { // Variable `e` is not used, prefixed with underscore
            // If alphanumeric parse fails, and it wasn't any of the above, then it's an error for single_key_unit
             Err(nom::Err::Error(nom::error::make_error(s, nom::error::ErrorKind::Alt)))
        }
    }
}

pub fn modifier(s: &str) -> IResult<&str, Modifier> {
    map_res(alpha1, Modifier::from_str)(s)
}

pub fn accord(s: &str) -> IResult<&str, Accord> {
    let (s_after_explicit_mods, explicit_modifiers) = fold_many0(
        terminated(modifier, nom_char('-')),
        Modifiers::empty,
        |mods, m| mods | m,
    )(s)?;

    // Handle cases like "ctrl-" or just "ctrl"
    if s_after_explicit_mods.is_empty() {
        return if explicit_modifiers.is_empty() && s.is_empty() { // Input was empty string
            Err(nom::Err::Error(nom::error::make_error(s, nom::error::ErrorKind::Eof)))
        } else if explicit_modifiers.is_empty() && !s.is_empty() { // Input was something like "!" or "-" or "a" that was not parsed as a modifier list.
                                                                // This will be handled by the `alt` below.
             Err(nom::Err::Error(nom::error::make_error(s, nom::error::ErrorKind::TakeTill1))) // Let alt try to parse it
        } else { // Input was like "ctrl-" or "shift-alt-"
             Ok(("", Accord::new(explicit_modifiers, None)))
        };
    }

    // Now, try to parse either a single_key_unit or a standalone modifier (e.g. "ctrl" in "ctrl-a,ctrl")
    alt((
        map(single_key_unit, move |unit: ParsedUnit| { // `move` for explicit_modifiers
            let (implicit_mod, code) = match unit {
                ParsedUnit::DirectCode(c) => (Modifiers::empty(), Some(c)),
                ParsedUnit::ShiftedCode(c) => (Modifier::Shift.into(), Some(c)),
                ParsedUnit::CustomCode(val) => (Modifiers::empty(), Some(Code::Custom(val))),
            };
            Accord::new(explicit_modifiers | implicit_mod, code)
        }),
        map(modifier, move |m: Modifier| { // `move` for explicit_modifiers. Handles cases like "ctrl-a,ctrl"
            // This branch is only taken if single_key_unit fails, which means s_after_explicit_mods is not a key.
            // So, it must be a standalone modifier.
            Accord::new(explicit_modifiers | m, None)
        }),
    ))(s_after_explicit_mods)
}

fn mouse_event(s: &str) -> IResult<&str, MouseEvent> {
    let button = alt((
        value(MouseButton::Left, alt((tag("click"), tag("lclick")))),
        value(MouseButton::Right, tag("rclick")),
        value(MouseButton::Middle, tag("mclick")),
    ));
    let buttons = map(separated_list1(nom_char('+'), button), MouseButtons::from_iter); // Use nom_char
    let click = map(buttons, MouseAction::Click);

    let wheel = alt((
        value(MouseAction::WheelUp, tag("wheelup")),
        value(MouseAction::WheelDown, tag("wheeldown")),
    ));

    let mut event = map(
        tuple((
            opt(terminated(mouse_modifier, nom_char('-'))), // Use nom_char
            alt((click, wheel)),
        )),
        |(modifier, action)| MouseEvent(action, modifier)
    );

    event(s)
}

pub fn r#macro(s: &str) -> IResult<&str, Macro> {
    let mut parser = alt((
        map(mouse_event, Macro::Mouse),
        map(media_code, Macro::Media),
        map(separated_list1(nom_char(','), accord), Macro::Keyboard), // Use nom_char
    ));
    parser(s)
}

pub fn address(s: &str) -> IResult<&str, (u8, u8)> {
    let byte = || map_res(digit1, u8::from_str);
    let mut parser = separated_pair(byte(), nom_char(':'), byte()); // Use nom_char
    parser(s)
}

/// Parses string with given parser ensuring that whole input is consumed.
pub fn parse<I, O, E, P>(parser: P, input: I) -> std::result::Result<O, E>
where
    I: InputLength,
    E: ParseError<I>,
    P: Parser<I, O, E>,
{
    use nom::Finish as _;
    all_consuming(parser)(input).finish().map(|(_, value)| value)
}

/// Parses string using given parser, as `parse` do, but also converts string reference
/// in returned error to String, so it may be used in implementations of `FromStr`.
pub fn from_str<O, P>(parser: P, s: &str) -> std::result::Result<O, nom::error::Error<String>>
where
    for <'a> P: Parser<&'a str, O, nom::error::Error<&'a str>>,
{
    match parse(parser, s) {
        Ok(value) => Ok(value),
        Err(nom::error::Error { input, code }) =>
            Err(nom::error::Error { input: input.to_owned(), code }),
    }
}

#[cfg(test)]
mod tests {
    use crate::keyboard::{Accord, Modifiers, Code, Modifier, Macro, MouseEvent, MouseModifier, MouseButton, MouseAction, MediaCode, WellKnownCode};
    // No longer need FromStr for Accord directly in tests if we use a helper
    // use std::str::FromStr;

    // Helper for tests to simplify parsing and error checking for accords
    fn parse_accord_str(s: &str) -> Result<Accord, nom::error::Error<String>> {
        super::from_str(super::accord, s)
    }

    #[test]
    fn parse_custom_code_via_unit() { // Updated test name for clarity
        // This test now uses the new accord parser which internally uses single_key_unit
        assert_eq!(parse_accord_str("<23>"), Ok(Accord::new(Modifiers::empty(), Some(Code::Custom(23)))));
        // Test that it doesn't consume trailing characters incorrectly
        assert!(parse_accord_str("<23>a").is_err() || parse_accord_str("<23>a").unwrap_err().input != "a");

    }

    #[test]
    fn parse_accord_basic() { // Renamed from parse_accord and expanded
        assert_eq!(parse_accord_str("a"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::A.into()))));
        // Test case sensitivity: "A" should also map to WellKnownCode::A and not imply shift. Shift is explicit.
        assert_eq!(parse_accord_str("A"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::A.into()))));
        assert_eq!(parse_accord_str("f1"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::F1.into()))));
        assert_eq!(parse_accord_str("enter"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Enter.into()))));
        assert_eq!(parse_accord_str("ctrl-a"), Ok(Accord::new(Modifier::Ctrl, Some(WellKnownCode::A.into()))));
        assert_eq!(parse_accord_str("win-ctrl-A"), Ok(Accord::new(Modifier::Win | Modifier::Ctrl, Some(WellKnownCode::A.into()))));
        assert_eq!(parse_accord_str("win-ctrl"), Ok(Accord::new(Modifier::Win | Modifier::Ctrl, None)));
        assert_eq!(parse_accord_str("shift-ctrl"), Ok(Accord::new(Modifier::Shift | Modifier::Ctrl, None)));
        assert_eq!(parse_accord_str("shift-<100>"), Ok(Accord::new(Modifier::Shift, Some(Code::Custom(100)))));

        // Original tests that should still pass (using .parse() which calls from_str -> accord)
        assert_eq!("A".parse(), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::A.into()))));
        assert_eq!("ctrl-A".parse(), Ok(Accord::new(Modifier::Ctrl, Some(WellKnownCode::A.into()))));
        assert_eq!("win-ctrl-A".parse(), Ok(Accord::new(Modifier::Win | Modifier::Ctrl, Some(WellKnownCode::A.into()))));
        assert_eq!("win-ctrl".parse(), Ok(Accord::new(Modifier::Win | Modifier::Ctrl, None)));
        assert_eq!("shift-<100>".parse(), Ok(Accord::new(Modifier::Shift, Some(Code::Custom(100)))));

        // Test error cases from original tests
        assert!("a1".parse::<Accord>().is_err()); // a1 is not a valid key name or structure
        assert!("a+".parse::<Accord>().is_err()); // a+ is not valid; '+' is for mouse buttons or shifted '='
    }

    #[test]
    fn parse_accord_special_chars_shifted() {
        assert_eq!(parse_accord_str("!"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::N1.into()))));
        assert_eq!(parse_accord_str("@"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::N2.into()))));
        assert_eq!(parse_accord_str("#"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::N3.into()))));
        assert_eq!(parse_accord_str("$"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::N4.into()))));
        assert_eq!(parse_accord_str("%"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::N5.into()))));
        assert_eq!(parse_accord_str("^"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::N6.into()))));
        assert_eq!(parse_accord_str("&"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::N7.into()))));
        assert_eq!(parse_accord_str("*"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::N8.into()))));
        assert_eq!(parse_accord_str("("), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::N9.into()))));
        assert_eq!(parse_accord_str(")"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::N0.into()))));
        assert_eq!(parse_accord_str("_"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::Minus.into()))));
        assert_eq!(parse_accord_str("+"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::Equal.into()))));
        assert_eq!(parse_accord_str("{"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::LeftBracket.into()))));
        assert_eq!(parse_accord_str("}"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::RightBracket.into()))));
        assert_eq!(parse_accord_str("|"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::Backslash.into()))));
        assert_eq!(parse_accord_str(":"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::Semicolon.into()))));
        assert_eq!(parse_accord_str("\""), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::Quote.into())))); // Double quote
        assert_eq!(parse_accord_str("~"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::Grave.into()))));
        // '<' and '>' are ambiguous with custom codes if not handled carefully.
        // SHIFTED_CHARS_MAP handles these as shifted comma/dot.
        assert_eq!(parse_accord_str("<"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::Comma.into()))));
        assert_eq!(parse_accord_str(">"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::Dot.into()))));
        assert_eq!(parse_accord_str("?"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::Slash.into()))));
    }

    #[test]
    fn parse_accord_special_chars_direct() {
        assert_eq!(parse_accord_str("-"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Minus.into()))));
        assert_eq!(parse_accord_str("="), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Equal.into()))));
        assert_eq!(parse_accord_str("["), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::LeftBracket.into()))));
        assert_eq!(parse_accord_str("]"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::RightBracket.into()))));
        assert_eq!(parse_accord_str("\\"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Backslash.into()))));
        assert_eq!(parse_accord_str(";"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Semicolon.into()))));
        assert_eq!(parse_accord_str("'"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Quote.into())))); // Single quote
        assert_eq!(parse_accord_str("`"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Grave.into()))));
        assert_eq!(parse_accord_str(","), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Comma.into()))));
        assert_eq!(parse_accord_str("."), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Dot.into()))));
        assert_eq!(parse_accord_str("/"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Slash.into()))));
        assert_eq!(parse_accord_str(" "), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Space.into()))));
    }

    #[test]
    fn parse_accord_combined_modifiers_and_special_chars() {
        assert_eq!(parse_accord_str("ctrl-!"), Ok(Accord::new(Modifier::Ctrl | Modifier::Shift, Some(WellKnownCode::N1.into()))));
        // Test explicit shift with a character that implies shift: shift + "!"
        // The explicit shift should be combined with the implicit shift.
        // Current logic: explicit_modifiers | implicit_mod. So Shift | Shift = Shift. Correct.
        assert_eq!(parse_accord_str("shift-!"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::N1.into()))));
        assert_eq!(parse_accord_str("alt-shift-_"), Ok(Accord::new(Modifier::Alt | Modifier::Shift, Some(WellKnownCode::Minus.into()))));
        // Test explicit modifier with a direct char: "ctrl" + "-"
        assert_eq!(parse_accord_str("ctrl--"), Ok(Accord::new(Modifier::Ctrl, Some(WellKnownCode::Minus.into()))));
        // Test explicit shift with a char that would be shifted by SHIFTED_CHARS_MAP: "shift" + "+"
        // The WellKnownCode::Equal is the base key for '+'. Shift is implied. Explicit shift is also present.
        assert_eq!(parse_accord_str("shift-+"), Ok(Accord::new(Modifier::Shift, Some(WellKnownCode::Equal.into()))));
    }

    #[test]
    fn parse_macro_with_special_chars() { // Renamed from parse_macro and expanded
        // Existing tests from parse_macro
        assert_eq!("A,B".parse(), Ok(Macro::Keyboard(vec![
            Accord::new(Modifiers::empty(), Some(WellKnownCode::A.into())),
            Accord::new(Modifiers::empty(), Some(WellKnownCode::B.into())),
        ])));
        assert_eq!("ctrl-A,alt-backspace".parse(), Ok(Macro::Keyboard(vec![
            Accord::new(Modifier::Ctrl, Some(WellKnownCode::A.into())),
            Accord::new(Modifier::Alt, Some(WellKnownCode::Backspace.into())),
        ])));

        // New tests for special characters in macros
        assert_eq!("ctrl-A,alt-!".parse(), Ok(Macro::Keyboard(vec![
            Accord::new(Modifier::Ctrl, Some(WellKnownCode::A.into())),
            Accord::new(Modifier::Alt | Modifier::Shift, Some(WellKnownCode::N1.into())),
        ])));
        assert_eq!("-,=,[,],{,},ctrl-_".parse::<Macro>(), Ok(Macro::Keyboard(vec![
            Accord::new(Modifiers::empty(), Some(WellKnownCode::Minus.into())),
            Accord::new(Modifiers::empty(), Some(WellKnownCode::Equal.into())),
            Accord::new(Modifiers::empty(), Some(WellKnownCode::LeftBracket.into())),
            Accord::new(Modifiers::empty(), Some(WellKnownCode::RightBracket.into())),
            Accord::new(Modifier::Shift, Some(WellKnownCode::LeftBracket.into())), // {
            Accord::new(Modifier::Shift, Some(WellKnownCode::RightBracket.into())), // }
            Accord::new(Modifier::Ctrl | Modifier::Shift, Some(WellKnownCode::Minus.into())), // ctrl-_
        ])));

        // Test with spaces and mixed types
        assert_eq!("ctrl-space,shift-/, ".parse::<Macro>(), Ok(Macro::Keyboard(vec![
            Accord::new(Modifier::Ctrl, Some(WellKnownCode::Space.into())),
            Accord::new(Modifier::Shift, Some(WellKnownCode::Slash.into())), // shift-/ is '?'
            Accord::new(Modifiers::empty(), Some(WellKnownCode::Space.into())),
        ])));
    }

    #[test]
    fn parse_macro_mouse_and_media_unchanged() { // Ensure existing macro types still parse correctly
        assert_eq!("click".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Click(MouseButton::Left.into()), None)
        )));
        assert_eq!("rclick".parse(), Ok(Macro::Mouse( // From original file, ensure still works
            MouseEvent(MouseAction::Click(MouseButton::Right.into()), None)
        )));
        assert_eq!("click+rclick".parse(), Ok(Macro::Mouse( // From original file
            MouseEvent(MouseAction::Click(MouseButton::Left | MouseButton::Right), None)
        )));
        assert_eq!("ctrl-wheelup".parse(), Ok(Macro::Mouse( // From original file
            MouseEvent(MouseAction::WheelUp, Some(MouseModifier::Ctrl))
        )));
        assert_eq!("ctrl-click".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Click(MouseButton::Left.into()), Some(MouseModifier::Ctrl))
        )));
        assert_eq!("play".parse(), Ok(Macro::Media(MediaCode::Play))); // From original parse_media
    }

    #[test]
    fn parse_invalid_accords() {
        assert!(parse_accord_str("a1").is_err()); // "a1" is not a valid key name
        assert!(parse_accord_str("f123").is_err()); // "f123" is not a valid F-key
        assert!(parse_accord_str("ctrl-alt").is_ok()); // This should be a valid accord: Modifiers only
        assert_eq!(parse_accord_str("ctrl-alt-"), Ok(Accord::new(Modifier::Ctrl | Modifier::Alt, None))); // Trailing dash
        assert!(parse_accord_str("-ctrl").is_err()); // Modifier cannot prefix like this
        assert!(parse_accord_str("!a").is_err()); // Should be "!,a" for two accords or "shift-a" if '!' was a modifier (it's not)
        assert!(parse_accord_str("ctrl-alt_").is_err()); // "alt_" is not a single unit, "ctrl-alt-_" would be
        assert!(parse_accord_str("<abc>").is_err()); // Custom codes are digits only
        assert!(parse_accord_str("<12").is_err());   // Custom codes need closing '>'
        assert!(parse_accord_str("shift-").is_ok()); // shift- is a valid accord (modifier only)
        assert_eq!(parse_accord_str("shift-"), Ok(Accord::new(Modifier::Shift, None)));
        assert!(parse_accord_str("-").is_ok()); // Just a minus sign
        assert_eq!(parse_accord_str("-"), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::Minus.into()))));
        assert!(parse_accord_str("").is_err()); // Empty string is not a valid accord
        assert!(parse_accord_str("ctrl-").is_ok());
         assert_eq!(parse_accord_str("ctrl-"), Ok(Accord::new(Modifier::Ctrl, None)));
    }
}
