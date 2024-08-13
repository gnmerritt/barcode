use rsbwapi::*;
use std::collections::HashSet;

pub struct BuildOrder {
    to_build: Vec<(UnitType, i32)>,
    placed_buildings: Vec<UnitType>,
    building_ids: HashSet<usize>,
}

impl BuildOrder {
    pub fn new() -> Self {
        BuildOrder {
            to_build: vec![
                // NB: these supplies written like they'd show up in build orders
                (UnitType::Zerg_Hatchery, 11),
                (UnitType::Zerg_Spawning_Pool, 10),
                (UnitType::Zerg_Extractor, 9),
                (UnitType::Zerg_Lair, 10),
                (UnitType::Zerg_Spire, 16),
            ],
            placed_buildings: vec![],
            building_ids: HashSet::new(),
        }
    }

    pub fn get_next_building(&mut self, game: &Game) -> Option<UnitType> {
        let self_ = game.self_().unwrap();
        let supply = self_.supply_used();
        if let Some((building, min_supply)) = self.to_build.first() {
            // remember that BW doubles supplies
            if supply >= 2 * min_supply {
                return Some(building.clone());
            } else {
                return None;
            }
        }
        None
    }

    pub fn placed_building(&mut self, building: UnitType) {
        // add to placed to we can keep track of its cost
        self.placed_buildings.push(building);
        // remove from build order queue
        if let Some((bt, s)) = self.to_build.pop() {
            if building != bt {
                self.to_build.push((bt, s));
            }
        }
    }

    // remove buildings that have begun construction from our placed list
    // so we don't double-count their cost
    pub fn check_placed_buildings(&mut self, game: &Game) {
        let in_progress = game
            .self_()
            .unwrap()
            .get_units()
            .into_iter()
            .filter(|u| u.get_type().is_building() && u.is_being_constructed());
        for building in in_progress {
            let id = building.get_id();
            if !self.building_ids.contains(&id) {
                self.building_ids.insert(id);
                let bt = building.get_type();
                let index = self.placed_buildings.iter().position(|t| *t == bt);
                if let Some(index) = index {
                    self.placed_buildings.swap_remove(index);
                }
            }
        }
    }

    pub fn spent_minerals(&self) -> i32 {
        self.placed_buildings
            .iter()
            .map(UnitType::mineral_price)
            .sum()
    }

    pub fn spent_gas(&self) -> i32 {
        self.placed_buildings.iter().map(UnitType::gas_price).sum()
    }
}
