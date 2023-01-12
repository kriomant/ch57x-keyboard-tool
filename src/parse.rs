use nom::{
    IResult,
    sequence::{tuple, terminated},
    multi::fold_many0,
    character::complete::{char, alpha1, satisfy},
    branch::alt, combinator::{map, map_res, recognize}, error::Error,
};

use crate::keyboard::{Accord, Modifier, Modifiers};

use std::str::FromStr;

pub fn parse_accord(s: &str) -> IResult<&str, Accord> {
    // Key code
    let code = alt((
        alpha1::<_, Error<_>>, // is either word line ""
        recognize(satisfy(|c| c.is_ascii_digit())),
    ));
    let code = map_res(code, FromStr::from_str);

    let modifier = map_res(alpha1, Modifier::from_str);
    let modifiers = fold_many0(
        terminated(modifier, char('-')),
        Modifiers::empty,
        |mods, m| { mods | m }
    );

    let mut accord = map(tuple((modifiers, code)), |t| t.into());
    accord(s)
}
