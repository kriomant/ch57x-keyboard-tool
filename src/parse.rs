///! Collection of NOM parsers for various things.
///! Generally only `parse` and `from_str` functions should be called
///! from outside of this module, they ensures that whole input is
///! consumed.
///! Other functions are composable parsers for use within this module
///! or as parameters for functions mentioned above.

use nom::{
    Parser, IResult, InputLength,
    branch::alt,
    sequence::{tuple, terminated, separated_pair},
    multi::separated_list1,
    bytes::complete::tag,
    character::complete::{char, alpha1, alphanumeric1, digit1},
    combinator::{map, map_res, opt, all_consuming, value},
    error::{ParseError, Error, FromExternalError, ErrorKind},
};

use crate::keyboard::{Accord, Modifier, Modifiers, Macro, MouseEvent, MouseModifier, MouseButton, MouseButtons, MouseAction, MediaCode, Code};

use std::str::FromStr;

fn mouse_modifier(s: &str) -> IResult<&str, MouseModifier> {
    map_res(alpha1, MouseModifier::from_str)(s)
}

fn media_code(s: &str) -> IResult<&str, MediaCode> {
    map_res(alpha1, MediaCode::from_str)(s)
}

pub fn accord(s: &str) -> IResult<&str, Accord> {
    let (s, parts) = separated_list1(char('-'), alphanumeric1)(s)?;
    let (mods, code) = match Code::from_str(parts[parts.len()-1]) {
        Ok(code) => (&parts[0..parts.len()-1], Some(code)),
        Err(_) => (&parts[..], None),
    };

    let modifiers = mods.iter()
        .map(|m| Modifier::from_str(m)
            .map_err(|e| nom::Err::Failure(Error::from_external_error(*m, ErrorKind::MapRes, e)))
        )
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .fold(Modifiers::empty(), |mods, m| mods | *m);
    Ok((s, Accord::new(modifiers, code)))
}

fn mouse_event(s: &str) -> IResult<&str, MouseEvent> {
    let button = alt((
        value(MouseButton::Left, alt((tag("click"), tag("lclick")))),
        value(MouseButton::Right, tag("rclick")),
        value(MouseButton::Middle, tag("mclick")),
    ));
    let buttons = map(separated_list1(char('+'), button), MouseButtons::from_iter);
    let click = map(buttons, MouseAction::Click);

    let wheel = alt((
        value(MouseAction::WheelUp, tag("wheelup")),
        value(MouseAction::WheelDown, tag("wheeldown")),
    ));

    let mut event = map(
        tuple((
            opt(terminated(mouse_modifier, char('-'))),
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
        map(separated_list1(char(','), accord), Macro::Keyboard),
    ));
    parser(s)
}

pub fn address(s: &str) -> IResult<&str, (u8, u8)> {
    let byte = || map_res(digit1, u8::from_str);
    let mut parser = separated_pair(byte(), char(':'), byte());
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
    use crate::keyboard::{Accord, Modifiers, Code, Modifier, Macro, MouseEvent, MouseModifier, MouseButton, MouseAction, MediaCode};

    #[test]
    fn parse_accord() {
        assert_eq!("A".parse(), Ok(Accord::new(Modifiers::empty(), Some(Code::A))));
        assert_eq!("a".parse(), Ok(Accord::new(Modifiers::empty(), Some(Code::A))));
        assert_eq!("f1".parse(), Ok(Accord::new(Modifiers::empty(), Some(Code::F1))));
        assert_eq!("ctrl-A".parse(), Ok(Accord::new(Modifier::Ctrl, Some(Code::A))));
        assert_eq!("win-ctrl-A".parse(), Ok(Accord::new(Modifier::Win | Modifier::Ctrl, Some(Code::A))));
        assert_eq!("win-ctrl".parse(), Ok(Accord::new(Modifier::Win | Modifier::Ctrl, None)));

        assert!("a1".parse::<Accord>().is_err());
        assert!("a+".parse::<Accord>().is_err());
    }

    #[test]
    fn parse_macro() {
        assert_eq!("A,B".parse(), Ok(Macro::Keyboard(vec![
            Accord::new(Modifiers::empty(), Some(Code::A)),
            Accord::new(Modifiers::empty(), Some(Code::B)),
        ])));
        assert_eq!("ctrl-A,alt-backspace".parse(), Ok(Macro::Keyboard(vec![
            Accord::new(Modifier::Ctrl, Some(Code::A)),
            Accord::new(Modifier::Alt, Some(Code::Backspace)),
        ])));
        assert_eq!("click".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Click(MouseButton::Left.into()), None)
        )));
        assert_eq!("click+rclick".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Click(MouseButton::Left | MouseButton::Right), None)
        )));
        assert_eq!("ctrl-wheelup".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::WheelUp, Some(MouseModifier::Ctrl))
        )));
        assert_eq!("ctrl-click".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Click(MouseButton::Left.into()), Some(MouseModifier::Ctrl))
        )));
    }

    #[test]
    fn parse_media() {
        assert_eq!("play".parse(), Ok(Macro::Media(MediaCode::Play)));
    }
}
