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
            ensure!(layer.buttons.len() == self.rows as usize, "Invalid number of button rows in layer {i}");
            ensure!(layer.buttons.iter().all(|row| row.len() == self.columns as usize), "Invalid number of button columns in layer {i}");
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

#[derive(Debug, Deserialize)]
pub struct Layer {
    pub buttons: Vec<Vec<Option<Macro>>>,
    pub knobs: Vec<Option<Macro>>,
}

pub struct FlatLayer {
    pub buttons: Vec<Option<Macro>>,
    pub knobs: Vec<Option<Macro>>,
}

fn reorient_grid<T: Clone>(orientation: Orientation, rows: usize, cols: usize, data: Vec<Vec<T>>) -> Vec<T> {
    // Transforms physical button position to virtual.
    let tr = match orientation {
        Orientation::Normal =>           |r, c, _rows, _cols| (r, c),
        Orientation::UpsideDown =>       |r, c,  rows,  cols| (rows-r-1, cols-c-1),
        Orientation::Clockwise =>        |r, c, _rows,  cols| (c, cols-r),
        Orientation::CounterClockwise => |r, c,  rows, _cols| (rows-c, r),
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
