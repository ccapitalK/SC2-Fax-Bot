use rust_sc2::prelude::*;

pub fn a_move(units: &Units, position: Point2, queue: bool) {
    for unit in units {
        unit.attack(Target::Pos(position), queue);
    }
}