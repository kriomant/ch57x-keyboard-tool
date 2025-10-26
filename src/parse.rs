///! Collection of NOM parsers for various things.
///! Generally only `parse` and `from_str` functions should be called
///! from outside of this module, they ensures that whole input is
///! consumed.
///! Other functions are composable parsers for use within this module
///! or as parameters for functions mentioned above.

use nom::{
    IResult, InputLength, Parser, branch::alt, bytes::complete::tag, character::complete::{alpha1, alphanumeric1, char, digit1}, combinator::{all_consuming, cut, map, map_res, opt, recognize, value}, error::ParseError, multi::{fold_many0, separated_list1}, sequence::{delimited, pair, separated_pair, terminated, tuple}
};

use crate::keyboard::{Accord, Code, Macro, MediaCode, Modifier, Modifiers, MouseAction, MouseButton, MouseButtons, MouseEvent, MouseModifier, ScrollDirection, WellKnownCode};

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

pub fn modifiers_prefix(s: &str) -> IResult<&str, Modifiers> {
    let mut parser = fold_many0(
        terminated(modifier, char('-')),
        Modifiers::empty,
        |mods, m| mods | m);
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
            modifiers_prefix,
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

pub fn delta(s: &str) -> IResult<&str, i8> {
    let mut parser = map_res(
        recognize(pair(opt(tag("-")), digit1)),
        str::parse::<i8>
    );
    parser(s)
}

fn mouse_event(s: &str) -> IResult<&str, MouseEvent> {
    let mouse_move = map(
        delimited(
            tag("move("),
            cut(separated_pair(delta, tag(","), delta)),
            tag(")")
        ),
        |(x,y)| MouseAction::Move(x, y),
    );

    let click = alt((
        value(MouseButton::Left, alt((tag("click"), tag("lclick")))),
        value(MouseButton::Right, tag("rclick")),
        value(MouseButton::Middle, tag("mclick")),
    ));
    let clicks = map(separated_list1(char('+'), click), MouseButtons::from_iter);
    let click_action = map(clicks, MouseAction::Click);

    let mouse_button = map_res(alpha1, MouseButton::from_str);
    let mouse_buttons = map(separated_list1(char('+'), mouse_button), MouseButtons::from_iter);
    let mouse_drag = map(
        delimited(
            tag("drag("),
            cut(tuple((
                terminated(mouse_buttons, tag(",")),
                terminated(delta, tag(",")),
                delta,
            ))),
            tag(")"),
        ),
        |(buttons, x, y)| MouseAction::Drag(buttons, x, y),
    );
    let scroll_direction = alt((
        value(ScrollDirection::Up, tag("wheelup")),
        value(ScrollDirection::Down, tag("wheeldown")),
    ));
    let scroll = map(
        scroll_direction,
        MouseAction::Scroll,
    );

    let mut event = map(
        tuple((
            opt(terminated(mouse_modifier, char('-'))),
            alt((click_action, scroll, mouse_move, mouse_drag)),
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
    use crate::keyboard::{Accord, Code, Macro, MediaCode, Modifier, Modifiers, MouseAction, MouseButton, MouseEvent, MouseModifier, ScrollDirection, WellKnownCode};

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
            MouseEvent(MouseAction::Scroll(ScrollDirection::Up), Some(MouseModifier::Ctrl))
        )));
        assert_eq!("ctrl-click".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Click(MouseButton::Left.into()), Some(MouseModifier::Ctrl))
        )));
    }

    #[test]
    fn parse_media() {
        assert_eq!("play".parse(), Ok(Macro::Media(MediaCode::Play)));
    }

    #[test]
    fn parse_mouse_move() {
        assert_eq!("move(1,2)".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Move(1, 2), None)
        )));
        assert_eq!("ctrl-move(-5,10)".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Move(-5, 10), Some(MouseModifier::Ctrl))
        )));
        assert_eq!("ctrl-move(-5,10)".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Move(-5, 10), Some(MouseModifier::Ctrl))
        )));
    }

    #[test]
    fn parse_mouse_drag() {
        assert_eq!("drag(left,1,2)".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Drag(MouseButton::Left.into(), 1, 2), None)
        )));
        assert_eq!("drag(left+right,5,-3)".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Drag(MouseButton::Left | MouseButton::Right, 5, -3), None)
        )));
        assert_eq!("ctrl-drag(middle,-10,15)".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Drag(MouseButton::Middle.into(), -10, 15), Some(MouseModifier::Ctrl))
        )));
        assert_eq!("shift-drag(left+middle,0,0)".parse(), Ok(Macro::Mouse(
            MouseEvent(MouseAction::Drag(MouseButton::Left | MouseButton::Middle, 0, 0), Some(MouseModifier::Shift))
        )));
    }
}
