use rsbwapi::*;

pub struct BuildOrder {
    to_build: Vec<(UnitType, i32)>,
}

impl BuildOrder {
    pub fn new() -> Self {
        BuildOrder {
            to_build: vec![
                (UnitType::Zerg_Hatchery, 11),
                (UnitType::Zerg_Spawning_Pool, 10),
                (UnitType::Zerg_Extractor, 9),
                (UnitType::Zerg_Lair, 10),
                (UnitType::Zerg_Spire, 16),
            ],
        }
    }

    pub fn get_next_building(&mut self, game: &Game) -> Option<UnitType> {
        let self_ = game.self_().unwrap();
        let supply = self_.supply_used();
        if let Some((building, min_supply)) = self.to_build.first() {
            if supply >= *min_supply {
                return Some(building.clone());
            } else {
                return None;
            }
        }
        None
    }

    pub fn placed_building(&mut self, building: UnitType) {
        if let Some((bt, s)) = self.to_build.pop() {
            if building != bt {
                self.to_build.push((bt, s));
            }
        }
    }
}
