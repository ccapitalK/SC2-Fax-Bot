use ndarray::Array2;

use rust_sc2::prelude::*;
use rust_sc2::pixel_map::{Pixel, PixelMap};
use rust_sc2::geometry::Rect;

fn trim_array<T: Clone + Default>(data: &Array2<T>, bounds: Rect) -> Array2<T> {
    let width = data.len_of(ndarray::Axis(0));
    let height = data.len_of(ndarray::Axis(1));
    assert!(width >= bounds.x1);
    assert!(height > bounds.y1);
    let shape = (bounds.x1 - bounds.x0, bounds.y1 - bounds.y0);
    let mut rv = Array2::<T>::default(shape);
    for x in bounds.x0..bounds.x1 {
        for y in bounds.y0..bounds.y1 {
            rv[(x - bounds.x0, y - bounds.y0)] = data[(x, y)].clone();
        }
    }
    rv
}

#[derive(Default)]
pub struct MapInfo {
    pub pathable_tiles: PixelMap,
    pub energy_map: Array2<f32>,
}

pub fn dump_pixel_map(map: &PixelMap) {
    for y in (0..(map.len_of(ndarray::Axis(1)))).rev() {
        for x in 0..(map.len_of(ndarray::Axis(0))) {
            print!("{}", if map[(x, y)] == Pixel::Set {
                "1"
            } else {
                "0"
            });
        }
        println!();
    }
}

impl MapInfo {
    pub fn new(pathable_tiles: &PixelMap, boundaries: Rect) -> Self {
        let pathable_tiles = trim_array(pathable_tiles, boundaries);
        let width = pathable_tiles.len_of(ndarray::Axis(0));
        let height = pathable_tiles.len_of(ndarray::Axis(1));
        let energy_map = Array2::<f32>::zeros((width, height));
        let map_info = MapInfo { pathable_tiles, energy_map };
        map_info.dump_pathable_tiles();
        map_info
    }
    pub fn dump_pathable_tiles(&self) {
        let width = self.pathable_tiles.len_of(ndarray::Axis(0));
        let height = self.pathable_tiles.len_of(ndarray::Axis(1));
        dump_pixel_map(&self.pathable_tiles);
    }
}