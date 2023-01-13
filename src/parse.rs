use nom::{
    IResult,
    sequence::{tuple, terminated, pair},
    multi::{fold_many0, separated_list1},
    character::complete::{char, alpha1, alphanumeric1},
    combinator::{map, map_res, complete, opt},
};

use crate::keyboard::{Accord, Modifier, Modifiers, Macro, MouseEvent, MouseModifier, MouseButton, MouseButtons};

use std::str::FromStr;

pub fn parse_mouse_modifier(s: &str) -> IResult<&str, MouseModifier> {
    map_res(alpha1, MouseModifier::from_str)(s)
}

pub fn parse_modifiers(s: &str) -> IResult<&str, Modifiers> {
    let modifier = map_res(alpha1, Modifier::from_str);
    let mut modifiers = fold_many0(
        terminated(modifier, char('-')),
        Modifiers::empty,
        |mods, m| { mods | m }
    );
    modifiers(s)
}

pub fn parse_accord(s: &str) -> IResult<&str, Accord> {
    // Key code
    let code = alphanumeric1;
    let code = map_res(code, FromStr::from_str);

    let accord = complete(tuple((parse_modifiers, code)));
    let mut accord = map(accord, |t| t.into());
    accord(s)
}

pub fn parse_mouse_event(s: &str) -> IResult<&str, MouseEvent> {
    use nom::branch::alt;
    use nom::combinator::value;
    use nom::bytes::complete::tag;

    let button = alt((
        value(MouseButton::Left, alt((tag("click"), tag("lclick")))),
        value(MouseButton::Right, tag("rclick")),
        value(MouseButton::Middle, tag("mclick")),
    ));
    let buttons = map(separated_list1(char('+'), button), MouseButtons::from_iter);
    let click = map(buttons, MouseEvent::Click);

    let wheel = alt((
        value(MouseEvent::WheelUp as fn(Option<MouseModifier>) -> MouseEvent, tag("wheelup")),
        value(MouseEvent::WheelDown as fn(Option<MouseModifier>) -> MouseEvent, tag("wheeldown")),
    ));

    let mut event = alt((
        click,
        map(
            pair(opt(terminated(parse_mouse_modifier, char('-'))), wheel),
            |(modifier, wheel)| wheel(modifier)
        ),
    ));

    event(s)
}

pub fn parse_macro(s: &str) -> IResult<&str, Macro> {
    use nom::branch::alt;
    let mut parser = alt((
        map(separated_list1(char(','), parse_accord), Macro::Keyboard),
        map(parse_mouse_event, Macro::Mouse),
    ));
    parser(s)
}

#[cfg(test)]
mod tests {
    use crate::{keyboard::{Accord, Modifiers, Code, Modifier, Macro, MouseEvent, MouseModifier, MouseButton}};

    #[test]
    fn parse_accord() {
        assert_eq!("A".parse(), Ok(Accord::new(Modifiers::empty(), Code::A)));
        assert_eq!("a".parse(), Ok(Accord::new(Modifiers::empty(), Code::A)));
        assert_eq!("f1".parse(), Ok(Accord::new(Modifiers::empty(), Code::F1)));
        assert_eq!("ctrl-A".parse(), Ok(Accord::new(Modifier::Ctrl, Code::A)));
        assert_eq!("win-ctrl-A".parse(), Ok(Accord::new(Modifier::Win | Modifier::Ctrl, Code::A)));

        assert!("a1".parse::<Accord>().is_err());
        assert!("a+".parse::<Accord>().is_err());
    }

    #[test]
    fn parse_macro() {
        assert_eq!("A,B".parse(), Ok(Macro::Keyboard(vec![
            Accord::new(Modifiers::empty(), Code::A),
            Accord::new(Modifiers::empty(), Code::B),
        ])));
        assert_eq!("ctrl-A,alt-backspace".parse(), Ok(Macro::Keyboard(vec![
            Accord::new(Modifier::Ctrl, Code::A),
            Accord::new(Modifier::Alt, Code::Backspace),
        ])));
        assert_eq!("click".parse(), Ok(Macro::Mouse(
            MouseEvent::Click(MouseButton::Left.into())
        )));
        assert_eq!("click+rclick".parse(), Ok(Macro::Mouse(
            MouseEvent::Click(MouseButton::Left | MouseButton::Right)
        )));
        assert_eq!("ctrl-wheelup".parse(), Ok(Macro::Mouse(
            MouseEvent::WheelUp(Some(MouseModifier::Ctrl))
        )));
    }
}
