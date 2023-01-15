use anyhow::{Result, ensure};
use serde::Deserialize;

use crate::keyboard::Macro;

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
    use super::{Config, reorient_grid, Orientation};

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
}
