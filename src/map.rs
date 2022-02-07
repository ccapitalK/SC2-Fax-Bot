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
    pub partitions: Array2<u16>,
    pub width: usize,
    pub height: usize,
    pub zero_offset: Point2,
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

fn partition(pathable: &PixelMap) -> Array2<u16> {
    let shape = (pathable.shape()[0], pathable.shape()[1]);
    let mut partition = Array2::zeros(shape);
    partition
}

pub struct TrimmedPoint {
    point: Point2,
}

impl MapInfo {
    pub fn new(pathable_tiles: &PixelMap, boundaries: Rect) -> Self {
        let pathable_tiles = trim_array(pathable_tiles, boundaries);
        let width = pathable_tiles.len_of(ndarray::Axis(0));
        let height = pathable_tiles.len_of(ndarray::Axis(1));
        let energy_map = Array2::<f32>::zeros((width, height));
        let partitions = partition(&pathable_tiles);
        let map_info = MapInfo {
            pathable_tiles,
            energy_map,
            width,
            height,
            partitions,
            zero_offset: Point2::new(boundaries.x0 as f32, boundaries.x1 as f32),
        };
        map_info
    }
    pub fn dump_pathable_tiles(&self) {
        dump_pixel_map(&self.pathable_tiles);
    }
    pub fn normalize_point(&self, mut point: Point2) -> TrimmedPoint {
        point = point - self.zero_offset;
        point.x = point.x.max(0.0).min(self.width as f32);
        point.y = point.y.max(0.0).min(self.height as f32);
        TrimmedPoint { point }
    }
    pub fn extract_point(&self, trimmed_point: TrimmedPoint) -> Point2 {
        trimmed_point.point + self.zero_offset
    }
    pub fn get_random_point(&self) -> Point2 {
        self.extract_point(TrimmedPoint {
            point: Point2 {
                x: rand::random::<f32>() * self.width as f32,
                y: rand::random::<f32>() * self.height as f32,
            }
        })
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::{BufReader, Read, Result};
    use rust_sc2::pixel_map::{PixelMap, Pixel};
    use ndarray::Array2;
    use crate::map::partition;

    fn read_map_from_bytes(data: &[u8]) -> PixelMap {
        let mut height = 0;
        let mut width = None;
        for line in data.split(|c| *c == b'\n').filter(|l| l.len() > 0) {
            match width {
                Some(w) => assert_eq!(line.len(), w),
                None => width = Some(line.len()),
            }
            height += 1;
        }
        let shape = (width.unwrap(), height);
        let mut partition = Array2::default(shape);
        for (y, line) in data.split(|c| *c == b'\n').filter(|l| l.len() > 0).enumerate() {
            for x in 0..width.unwrap() {
                partition[(x, y)] = if line[x] == b'0' {
                    Pixel::Empty
                } else {
                    Pixel::Set
                };
            }
        }
        partition
    }

    #[test]
    fn read_map() -> Result<()> {
        let mut data = vec![];
        let mut file = BufReader::new(File::open("tests/map1.txt")?);
        file.read_to_end(&mut data).unwrap();
        let map = read_map_from_bytes(&data);
        let partition_map = partition(&map);
        println!("{:#?}", partition_map);
        Ok(())
    }
}