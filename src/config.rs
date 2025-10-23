use anyhow::{bail, ensure, Result};
use serde::Deserialize;

use crate::keyboard::{Macro, KeyboardPart, MouseAction, MouseEvent};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub orientation: Orientation,
    pub rows: u8,
    pub columns: u8,
    pub knobs: u8,

    pub layers: Vec<Layer>,
}

impl Config {
    /// Validates config and renders it to flat list of macros for buttons
    /// and knobs taking orientation into account.
    pub fn render(self) -> Result<Vec<FlatLayer>> {
        // 3x1 keys + 1 knob keyboard has some limitations we need to check.
        let is_limited = (self.rows == 1 || self.columns == 1) && self.knobs == 1;

        self.layers.into_iter().enumerate().map(|(i, layer)| {
            let (orows, ocols) = if self.orientation.is_horizontal() {
                (self.rows, self.columns)
            } else {
                (self.columns, self.rows)
            };
            ensure!(layer.buttons.len() == orows as usize, "Invalid number of button rows in layer {i}");
            ensure!(layer.buttons.iter().all(|row| row.len() == ocols as usize), "Invalid number of button columns in layer {i}");
            ensure!(layer.knobs.len() == self.knobs as usize, "Invalid number of knobs in layer {i}");

            let buttons = reorient_grid(self.orientation, self.rows as usize, self.columns as usize, layer.buttons);
            let knobs = reorient_row(self.orientation, layer.knobs);

            if is_limited {
                let macro_with_modifiers_beside_first_key = buttons.iter().flatten().find(|macro_| {
                    match macro_ {
                        Macro::Keyboard(parts) => parts.iter().filter_map(|p| match p { KeyboardPart::Key(a) => Some(a), _ => None }).skip(1).any(|accord| !accord.modifiers.is_empty()),
                        _ => false,
                    }
                });
                if let Some(macro_) = macro_with_modifiers_beside_first_key {
                    bail!("1-row keyboard with 1 knob can handle modifiers for first key in sequence only: {}", macro_);
                }
            }

            // Validate delay usage: at most one delay allowed and if present it must be the first item
            for (r_idx, button_macro) in buttons.iter().enumerate() {
                if let Some(m) = button_macro {
                    // Validate mouse moves as well as keyboard parts
                    if let Macro::Mouse(MouseEvent(action, _)) = m {
                        if let MouseAction::Move { dx, dy } = action {
                            if *dx < -128 || *dx > 127 || *dy < -128 || *dy > 127 {
                                bail!("Invalid mapping: mouse move dx/dy ({},{}) exceeds supported range -128..127 in macro '{}' in layer {}, button index {}.", dx, dy, m, i, r_idx);
                            }
                        }
                    }

                    if let Macro::Keyboard(parts) = m {
                        // count delays
                        let delay_count = parts.iter().filter(|p| matches!(p, KeyboardPart::Delay(_))).count();
                        if delay_count > 1 {
                            bail!("Invalid mapping: more than one delay found in macro '{}' in layer {}, button index {}. Only a single leading delay is allowed.", m, i, r_idx);
                        }
                        if delay_count == 1 {
                            // ensure it is the first element
                            match parts.first() {
                                Some(KeyboardPart::Delay(ms)) => {
                                    if *ms > 6000 {
                                        bail!("Invalid mapping: delay {}ms exceeds maximum supported 6000ms in macro '{}' in layer {}, button index {}.", ms, m, i, r_idx);
                                    }
                                }
                                _ => {
                                    bail!("Invalid mapping: delay must be the first item in macro '{}' in layer {}, button index {}.", m, i, r_idx);
                                }
                            }
                        }
                    }
                }
            }

            // Validate knobs too (each knob has ccw/press/cw macros)
            for (k_idx, knob) in knobs.iter().enumerate() {
                let check = |opt_macro: &Option<Macro>| -> Result<()> {
                    if let Some(m) = opt_macro {
                        // Validate mouse move values on knobs too
                        if let Macro::Mouse(MouseEvent(action, _)) = m {
                            if let MouseAction::Move { dx, dy } = action {
                                if *dx < -128 || *dx > 127 || *dy < -128 || *dy > 127 {
                                    bail!("Invalid mapping: mouse move dx/dy ({},{}) exceeds supported range -128..127 in knob macro '{}' in layer {}, knob index {}.", dx, dy, m, i, k_idx);
                                }
                            }
                        }

                        if let Macro::Keyboard(parts) = m {
                            let delay_count = parts.iter().filter(|p| matches!(p, KeyboardPart::Delay(_))).count();
                            if delay_count > 1 {
                                bail!("Invalid mapping: more than one delay found in knob macro '{}' in layer {}, knob index {}. Only a single leading delay is allowed.", m, i, k_idx);
                            }
                            if delay_count == 1 {
                                match parts.first() {
                                    Some(KeyboardPart::Delay(ms)) => {
                                        if *ms > 6000 {
                                            bail!("Invalid mapping: delay {}ms exceeds maximum supported 6000ms in knob macro '{}' in layer {}, knob index {}.", ms, m, i, k_idx);
                                        }
                                    }
                                    _ => {
                                        bail!("Invalid mapping: delay must be the first item in knob macro '{}' in layer {}, knob index {}.", m, i, k_idx);
                                    }
                                }
                            }
                        }
                    }
                    Ok(())
                };

                check(&knob.ccw)?;
                check(&knob.press)?;
                check(&knob.cw)?;
            }

            Ok(FlatLayer { buttons, knobs })
        }).collect()
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all="lowercase")]
pub enum Orientation {
    Normal,
    UpsideDown,
    Clockwise,
    CounterClockwise,
}

impl Orientation {
    pub fn is_horizontal(self) -> bool {
        self == Orientation::Normal || self == Orientation::UpsideDown
    }
}

#[derive(Debug, Deserialize)]
pub struct Layer {
    pub buttons: Vec<Vec<Option<Macro>>>,
    pub knobs: Vec<Knob>,
}

#[derive(Debug, Deserialize)]
pub struct Knob {
    pub ccw: Option<Macro>,
    pub press: Option<Macro>,
    pub cw: Option<Macro>,
}

pub struct FlatLayer {
    pub buttons: Vec<Option<Macro>>,
    pub knobs: Vec<Knob>,
}

fn reorient_grid<T: Clone>(orientation: Orientation, rows: usize, cols: usize, data: Vec<Vec<T>>) -> Vec<T> {
    // Transforms physical button position to virtual.
    let tr = match orientation {
        Orientation::Normal =>           |r, c, _rows, _cols| (r, c),
        Orientation::UpsideDown =>       |r, c,  rows,  cols| (rows-r-1, cols-c-1),
        Orientation::Clockwise =>        |r, c,  rows, _cols| (c, rows-r-1),
        Orientation::CounterClockwise => |r, c, _rows,  cols| (cols-c-1, r),
    };
    (0..rows*cols).map(|i| {
        let (r, c) = tr(i / cols, i % cols, rows, cols);
        data[r][c].clone()
    }).collect()
}

fn reorient_row<T>(orientation: Orientation, mut data: Vec<T>) -> Vec<T> {
    let reverse = match orientation {
        Orientation::Normal => false,
        Orientation::UpsideDown => true,
        Orientation::Clockwise => true,
        Orientation::CounterClockwise => false,
    };
    if reverse {
        data.reverse();
    }
    data
}

#[cfg(test)]
mod tests {
    use crate::config::Layer;

    use super::{reorient_grid, Config, Knob, Orientation};

    use std::path::PathBuf;

    #[test]
    fn parse_example_config() -> anyhow::Result<()> {
        let mut path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        path.push("example-mapping.yaml");
        let file = std::fs::File::open(&path)?;

        // Load and validate mapping.
        let config: Config = serde_yaml::from_reader(file)?;
        config.render()?;
        Ok(())
    }

    #[test]
    fn test_reorient_grid() {
        assert_eq!(
            reorient_grid(Orientation::Normal, 2, 3, vec![
                vec![1, 2, 3],
                vec![4, 5, 6],
            ]),
            vec![1, 2, 3, 4, 5, 6],
        );
        assert_eq!(
            reorient_grid(Orientation::UpsideDown, 2, 3, vec![
                vec![1, 2, 3],
                vec![4, 5, 6],
            ]),
            vec![6, 5, 4, 3, 2, 1],
        );
        assert_eq!(
            reorient_grid(Orientation::Clockwise, 2, 3, vec![
                vec![1, 2],
                vec![3, 4],
                vec![5, 6],
            ]),
            vec![2, 4, 6, 1, 3, 5],
        );
        assert_eq!(
            reorient_grid(Orientation::CounterClockwise, 2, 3, vec![
                vec![1, 2],
                vec![3, 4],
                vec![5, 6],
            ]),
            vec![5, 3, 1, 6, 4, 2],
        );
    }

    #[test]
    #[should_panic(expected="can handle modifiers for first key in sequence only")]
    fn test_limited_keyboard() {
        let config = Config {
            orientation: Orientation::Normal,
            rows: 1,
            columns: 3,
            knobs: 1,
            layers: vec![
                Layer {
                    buttons: vec![
                        vec![
                            Some("a,alt-b".parse().unwrap()),
                            None,
                            None
                        ],
                    ],
                    knobs: vec![Knob { ccw: None, press: None, cw: None }],
                },
            ],
        };
        config.render().unwrap();
    }

    #[test]
    fn accept_single_leading_delay() {
        let config = Config {
            orientation: Orientation::Normal,
            rows: 1,
            columns: 3,
            knobs: 0,
            layers: vec![
                Layer {
                    buttons: vec![vec![
                        Some("delay[1000],1,a,b,c".parse().unwrap()),
                        None,
                        None
                    ]],
                    knobs: vec![],
                }
            ],
        };
        config.render().unwrap();
    }

    #[test]
    #[should_panic(expected="more than one delay")]
    fn reject_multiple_delays() {
        let config = Config {
            orientation: Orientation::Normal,
            rows: 1,
            columns: 3,
            knobs: 0,
            layers: vec![
                Layer {
                    buttons: vec![vec![
                        Some("delay[1000],delay[200],1".parse().unwrap()),
                        None,
                        None
                    ]],
                    knobs: vec![],
                }
            ],
        };
        config.render().unwrap();
    }

    #[test]
    #[should_panic(expected="delay must be the first")]
    fn reject_non_leading_delay() {
        let config = Config {
            orientation: Orientation::Normal,
            rows: 1,
            columns: 3,
            knobs: 0,
            layers: vec![
                Layer {
                    buttons: vec![vec![
                        Some("1,delay[100],a".parse().unwrap()),
                        None,
                        None
                    ]],
                    knobs: vec![],
                }
            ],
        };
        config.render().unwrap();
    }

    #[test]
    fn accept_knob_leading_delay() {
        let config = Config {
            orientation: Orientation::Normal,
            rows: 1,
            columns: 1,
            knobs: 1,
            layers: vec![
                Layer {
                    buttons: vec![vec![None]],
                    knobs: vec![Knob { ccw: Some("delay[500],1".parse().unwrap()), press: None, cw: None }],
                }
            ],
        };
        config.render().unwrap();
    }

    #[test]
    #[should_panic(expected="more than one delay")]
    fn reject_knob_multiple_delays() {
        let config = Config {
            orientation: Orientation::Normal,
            rows: 1,
            columns: 1,
            knobs: 1,
            layers: vec![
                Layer {
                    buttons: vec![vec![None]],
                    knobs: vec![Knob { ccw: Some("delay[100],delay[200],1".parse().unwrap()), press: None, cw: None }],
                }
            ],
        };
        config.render().unwrap();
    }

    #[test]
    #[should_panic(expected="delay must be the first")]
    fn reject_knob_non_leading_delay() {
        let config = Config {
            orientation: Orientation::Normal,
            rows: 1,
            columns: 1,
            knobs: 1,
            layers: vec![
                Layer {
                    buttons: vec![vec![None]],
                    knobs: vec![Knob { ccw: Some("1,delay[100]".parse().unwrap()), press: None, cw: None }],
                }
            ],
        };
        config.render().unwrap();
    }
}
