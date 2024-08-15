use rsbwapi::*;
use std::collections::{HashSet, VecDeque};

pub struct BuildOrder {
    to_build: VecDeque<(UnitType, i32)>,
    placed_buildings: Vec<UnitType>,
    building_ids: HashSet<usize>,
}

impl BuildOrder {
    pub fn new() -> Self {
        BuildOrder {
            to_build: VecDeque::from([
                // NB: these supplies written like they'd show up in build orders
                (UnitType::Zerg_Hatchery, 11),
                (UnitType::Zerg_Spawning_Pool, 10),
                (UnitType::Zerg_Extractor, 9),
                (UnitType::Zerg_Lair, 10),
                (UnitType::Zerg_Spire, 20),
                (UnitType::Zerg_Hatchery, 30),
                (UnitType::Zerg_Hatchery, 50),
                (UnitType::Zerg_Queens_Nest, 50),
                (UnitType::Zerg_Hive, 50),
                (UnitType::Zerg_Hatchery, 70),
                (UnitType::Zerg_Hatchery, 90),
            ]),
            placed_buildings: vec![],
            building_ids: HashSet::new(),
        }
    }

    pub fn get_next_building(&self, supply_used: i32) -> Option<UnitType> {
        if let Some((building, min_supply)) = self.to_build.front() {
            // remember that BW doubles supplies
            if supply_used >= 2 * min_supply {
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
        if let Some((bt, _)) = self.to_build.front() {
            if building == *bt {
                self.to_build.pop_front();
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_building() {
        let mut bo = BuildOrder::new();
        assert_eq!(bo.get_next_building(8), None, "saw building too early");
        assert_eq!(
            bo.get_next_building(22),
            Some(UnitType::Zerg_Hatchery),
            "got hatch first"
        );
        // no-op to place a building not in the order
        bo.placed_building(UnitType::Terran_Barracks);
        assert_eq!(
            bo.get_next_building(22),
            Some(UnitType::Zerg_Hatchery),
            "still got hatch"
        );
        assert_eq!(
            bo.spent_minerals(),
            UnitType::Terran_Barracks.mineral_price(),
            "barracks mineral price accounted for if we say we placed it"
        );

        bo.placed_building(UnitType::Zerg_Hatchery);
        assert_eq!(
            bo.get_next_building(22),
            Some(UnitType::Zerg_Spawning_Pool),
            "pool after hatch"
        );
    }

    #[test]
    fn test_spent_resources() {
        let mut bo = BuildOrder::new();
        bo.placed_building(UnitType::Zerg_Hatchery);
        assert_eq!(bo.spent_minerals(), UnitType::Zerg_Hatchery.mineral_price());
        assert_eq!(bo.spent_gas(), UnitType::Zerg_Hatchery.gas_price());
    }
}
