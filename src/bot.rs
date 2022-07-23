use rust_sc2::prelude::*;

use crate::state::{BotState, GetBotState};
use float_ord::FloatOrd;

#[bot]
#[derive(Default)]
pub struct FaxBot {
    pub state: BotState,
}

impl GetBotState for FaxBot {
    fn get_state(&self) -> &BotState {
        &self.state
    }
    fn get_state_mut(&mut self) -> &mut BotState {
        &mut self.state
    }
}

impl FaxBot {
    pub fn energy_cost(&self, ability: AbilityId) -> Option<usize> {
        // FIXME: You'd expect something like `Some(self.game_data.abilities.get(&ability)?.energy_cost)`
        //        to work, but apparently the SC2 API doesn't provide ability energy costs anywhere :(
        match ability {
            AbilityId::EffectInjectLarva => Some(25),
            _ => unimplemented!(),
        }
    }
    pub fn count_unit(&self, building_id: UnitTypeId) -> usize {
        self.counter().count(building_id) + self.counter().ordered().count(building_id)
    }
}

impl Player for FaxBot {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings::new(Race::Zerg)
    }
    fn on_start(&mut self) -> SC2Result<()> {
        let start_location = self.start_location;
        let enemy_starts = self
            .game_info
            .start_locations
            .iter()
            .copied()
            .filter(|p| *p != start_location)
            .collect::<Vec<_>>();
        let mut points = self
            .expansions
            .iter()
            .map(|e| e.loc)
            .filter(|p| *p != start_location)
            .collect::<Vec<_>>();
        points.sort_by_cached_key(|p| enemy_starts.iter().map(|s| FloatOrd(p.distance(s))).min());
        self.state.desired_workers = 38;
        self.state.desired_gasses = 2;
        self.state.desired_bases = 2;
        self.state.micro.enemy_base_locations_by_expansion_order = points;
        self.state.map_info =
            crate::map::MapInfo::new(&self.game_info.pathing_grid, self.game_info.playable_area);
        println!("Started bot");
        Ok(())
    }
    fn on_step(&mut self, iteration: usize) -> SC2Result<()> {
        self.determine_state_for_tick(iteration);
        if !self.perform_building(iteration)? {
            self.perform_training(iteration)?;
        }
        self.perform_micro(iteration)
    }
    fn on_end(&self, _result: GameResult) -> SC2Result<()> {
        println!("Finished bot");
        Ok(())
    }
    fn on_event(&mut self, event: Event) -> SC2Result<()> {
        let army_types = [
            UnitTypeId::Zergling,
            UnitTypeId::Roach,
            UnitTypeId::Hydralisk,
        ];
        if let Event::UnitCreated(tag) = event {
            if let Some(unit) = self.units.my.units.get(tag) {
                if army_types.contains(&unit.type_id()) {
                    let rally = self.get_rally_point();
                    unit.attack(Target::Pos(rally), false);
                }
            }
        };
        Ok(())
    }
}
