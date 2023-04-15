///! Collection of NOM parsers for various things.
///! Generally only `parse` and `from_str` functions should be called
///! from outside of this module, they ensures that whole input is
///! consumed.
///! Other functions are composable parsers for use within this module
///! or as parameters for functions mentioned above.

use nom::{
    Parser, IResult, InputLength,
    branch::alt,
    sequence::{tuple, terminated, separated_pair, delimited, pair},
    multi::{separated_list1, fold_many0},
    bytes::complete::tag,
    character::complete::{char, alpha1, alphanumeric1, digit1},
    combinator::{map, map_res, opt, all_consuming, value},
    error::ParseError,
};

use crate::keyboard::{Accord, Modifier, Modifiers, Macro, MouseEvent, MouseModifier, MouseButton, MouseButtons, MouseAction, MediaCode, Code, WellKnownCode};

use std::str::FromStr;

fn mouse_modifier(s: &str) -> IResult<&str, MouseModifier> {
    map_res(alpha1, MouseModifier::from_str)(s)
}

fn media_code(s: &str) -> IResult<&str, MediaCode> {
    map_res(alpha1, MediaCode::from_str)(s)
}

pub fn code(s: &str) -> IResult<&str, Code> {
    let mut parser = alt((
        map(
            delimited(char('<'),
                      map_res(digit1, str::parse),
                      char('>')),
            Code::Custom),
        map_res(alphanumeric1,
                |word| WellKnownCode::from_str(word).map(Code::WellKnown)),
    ));
    parser(s)
}

pub fn modifier(s: &str) -> IResult<&str, Modifier> {
    let mut parser = map_res(alpha1, Modifier::from_str);
    parser(s)
}

pub fn accord(s: &str) -> IResult<&str, Accord> {
    enum Fix { Modifier(Modifier), Code(Code) }

    let mut parser = alt((
        // <code>
        map(code,
            |code| Accord::new(Modifiers::empty(), Some(code))),

        // (<modifier> '-')* (<code>|<modifier>)?
        map(pair(
            fold_many0(terminated(modifier, char('-')),
                       Modifiers::empty,
                       |mods, m| mods | m),
            alt((
                map(code, Fix::Code),
                map(modifier, Fix::Modifier),
            )),
        ), |(mods, fix)| match fix {
            Fix::Code(code) => Accord::new(mods, Some(code)),
            Fix::Modifier(m) => Accord::new(mods | m, None),
        })
    ));
    parser(s)
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
    use crate::keyboard::{Accord, Modifiers, Code, Modifier, Macro, MouseEvent, MouseModifier, MouseButton, MouseAction, MediaCode, WellKnownCode};

    #[test]
    fn parse_custom_code() {
        assert_eq!("<23>".parse(), Ok(Code::Custom(23)));
    }

    #[test]
    fn parse_accord() {
        assert_eq!("A".parse(), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::A.into()))));
        assert_eq!("a".parse(), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::A.into()))));
        assert_eq!("f1".parse(), Ok(Accord::new(Modifiers::empty(), Some(WellKnownCode::F1.into()))));
        assert_eq!("ctrl-A".parse(), Ok(Accord::new(Modifier::Ctrl, Some(WellKnownCode::A.into()))));
        assert_eq!("win-ctrl-A".parse(), Ok(Accord::new(Modifier::Win | Modifier::Ctrl, Some(WellKnownCode::A.into()))));
        assert_eq!("win-ctrl".parse(), Ok(Accord::new(Modifier::Win | Modifier::Ctrl, None)));
        assert_eq!("shift-<100>".parse(), Ok(Accord::new(Modifier::Shift, Some(Code::Custom(100)))));

        assert!("a1".parse::<Accord>().is_err());
        assert!("a+".parse::<Accord>().is_err());
    }

    #[test]
    fn parse_macro() {
        assert_eq!("A,B".parse(), Ok(Macro::Keyboard(vec![
            Accord::new(Modifiers::empty(), Some(WellKnownCode::A.into())),
            Accord::new(Modifiers::empty(), Some(WellKnownCode::B.into())),
        ])));
        assert_eq!("ctrl-A,alt-backspace".parse(), Ok(Macro::Keyboard(vec![
            Accord::new(Modifier::Ctrl, Some(WellKnownCode::A.into())),
            Accord::new(Modifier::Alt, Some(WellKnownCode::Backspace.into())),
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
