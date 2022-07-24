use rust_sc2::prelude::*;

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default)]
pub struct ObjectSpotted {
    pub position: Point2,
    pub timestamp: usize,
}

#[derive(Debug)]
pub struct ObjectPermanence<T: std::fmt::Debug> {
    pub map: HashMap<u64, (ObjectSpotted, T)>,
}

impl<T: std::fmt::Debug> Default for ObjectPermanence<T> {
    fn default() -> Self {
        ObjectPermanence {
            map: HashMap::new(),
        }
    }
}

impl<T: std::fmt::Debug> ObjectPermanence<T> {
    pub fn update_all<F>(&mut self, timestamp: usize, units: &Units, f: F)
        where
            F: Fn(&Unit) -> T,
    {
        for unit in units {
            let tag = unit.tag();
            let val = f(unit);
            let spotted = ObjectSpotted {
                position: unit.position(),
                timestamp,
            };
            self.map.insert(tag, (spotted, val));
        }
    }
}

#[derive(Debug, Default)]
pub struct BotState {
    pub bases: Vec<Point2>,
    pub expansion_order: Vec<rust_sc2::bot::Expansion>,
    pub peak_army: usize,
    pub desired_workers: usize,
    pub desired_gasses: usize,
    pub desired_bases: usize,
    pub is_under_attack: bool,
    pub micro: crate::micro::MicroState,
    pub map_info: crate::map::MapInfo,
    pub my_structures: ObjectPermanence<()>,
    pub enemy_units: ObjectPermanence<UnitTypeId>,
}

impl BotState {
    pub fn update_my_recent_structure_positions(&mut self, structures: &Units, iteration: usize) {
        self.my_structures.update_all(iteration, structures, |_| ());
    }
    pub fn update_recent_enemy_spotted_information(
        &mut self,
        structures: &Units,
        iteration: usize,
    ) {
        self.enemy_units
            .update_all(iteration, structures, |u| u.type_id());
    }
    pub fn get_my_recent_structure_positions(&self, iteration: usize) -> Vec<Point2> {
        let recent_tick_threshold = 22 * 40;
        self.my_structures
            .map
            .iter()
            .filter_map(|(_, (o, _))| {
                (o.timestamp + recent_tick_threshold >= iteration).then_some(o.position)
            })
            .collect()
    }
    pub fn get_recent_enemy_spotted_information(
        &self,
        iteration: usize,
    ) -> Vec<(Point2, UnitTypeId)> {
        let recent_tick_threshold = 22 * 40;
        self.enemy_units
            .map
            .iter()
            .filter_map(|(_, &(o, t))| {
                (o.timestamp + recent_tick_threshold >= iteration)
                    .then_some((o.position, t))
            })
            .collect()
    }
    pub fn register_unit_destroyed(&mut self, tag: u64) {
        self.enemy_units.map.remove(&tag);
    }
}

pub trait GetBotState {
    fn get_state(&self) -> &BotState;
    fn get_state_mut(&mut self) -> &mut BotState;
}
