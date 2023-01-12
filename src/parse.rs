use nom::{
    IResult,
    sequence::{tuple, terminated},
    multi::{fold_many0, separated_list1},
    character::complete::{char, alpha1, alphanumeric1},
    combinator::{map, map_res, complete},
};

use crate::keyboard::{Accord, Modifier, Modifiers, Macro};

use std::str::FromStr;

pub fn parse_accord(s: &str) -> IResult<&str, Accord> {
    // Key code
    let code = alphanumeric1;
    let code = map_res(code, FromStr::from_str);

    let modifier = map_res(alpha1, Modifier::from_str);
    let modifiers = fold_many0(
        terminated(modifier, char('-')),
        Modifiers::empty,
        |mods, m| { mods | m }
    );

    let accord = complete(tuple((modifiers, code)));
    let mut accord = map(accord, |t| t.into());
    accord(s)
}

pub fn parse_macro(s: &str) -> IResult<&str, Macro> {
    let mut parser = map(separated_list1(char(','), parse_accord), |accords| Macro::Keyboard(accords));
    parser(s)
}

#[cfg(test)]
mod tests {
    use crate::keyboard::{Accord, Modifiers, Code, Modifier, Macro};

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
    }
}
