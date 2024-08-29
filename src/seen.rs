use rsbwapi::{Game, ScaledPosition, TilePosition, Unit, UnitId, UnitType};
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq)]
pub(crate) struct SeenUnit {
    pub unit_type: UnitType,
    pub position: ScaledPosition<1>,
    pub hp: i32,
    pub shields: i32,
    pub frame: i32,
}

impl SeenUnit {
    fn new(unit: &Unit, frame: i32) -> Self {
        SeenUnit {
            unit_type: unit.get_type(),
            position: unit.get_position(),
            hp: unit.get_hit_points(),
            shields: unit.get_shields(),
            frame,
        }
    }
}

// Keep track of things we see that don't belong to us
pub(crate) struct HaveSeen {
    gas_locs: HashSet<TilePosition>,
    minerals: HashSet<TilePosition>,
    units: HashMap<UnitId, SeenUnit>,
    buildings: HashMap<UnitId, SeenUnit>,
    building_counts: HashMap<UnitType, i8>,
}

impl HaveSeen {
    pub fn new() -> Self {
        HaveSeen {
            gas_locs: HashSet::new(),
            minerals: HashSet::new(),
            units: HashMap::new(),
            buildings: HashMap::new(),
            building_counts: HashMap::new(),
        }
    }

    pub fn on_frame(&mut self, _game: &Game) {
        // TODO: update frame counts
    }

    pub fn on_unit_discover(&mut self, game: &Game, unit: &Unit) {
        if let Some(self_) = game.self_() {
            if unit.get_player() == self_ {
                return;
            }
            let enemy = game.enemy();
            let frame = game.get_frame_count();
            let type_ = unit.get_type();
            match type_ {
                UnitType::Unknown => {}
                _ if type_.is_mineral_field() => {
                    self.minerals.insert(unit.get_tile_position().clone());
                }
                UnitType::Resource_Vespene_Geyser => {
                    self.gas_locs.insert(unit.get_tile_position().clone());
                }
                _ if type_.is_neutral() => {}
                _ if type_.is_refinery() => {
                    self.gas_locs.insert(unit.get_tile_position().clone());
                }
                _ if type_.is_building() => {
                    if Some(unit.get_player()) == enemy {
                        self.buildings
                            .insert(unit.get_id(), SeenUnit::new(unit, frame));
                    }
                }
                _ => {
                    if Some(unit.get_player()) == enemy {
                        self.units.insert(unit.get_id(), SeenUnit::new(unit, frame));
                    }
                }
            }
        }
    }

    pub fn on_unit_destroy(&mut self, id: UnitId) {
        self.units.remove(&id);
        if let Some(b) = self.buildings.remove(&id) {
            self.building_counts
                .entry(b.unit_type)
                .and_modify(|c| *c -= 1);
        }
    }

    pub fn get_gas_locs(&self) -> Vec<&TilePosition> {
        self.gas_locs.iter().collect()
    }

    pub fn get_mineral_locs(&self) -> Vec<&TilePosition> {
        self.minerals.iter().collect()
    }

    pub fn get_enemy_building(&self) -> Option<&SeenUnit> {
        self.buildings.values().next()
    }

    pub fn get_enemy_army(&self) -> Vec<&SeenUnit> {
        self.units
            .values()
            .filter(|su| !su.unit_type.is_worker() && su.unit_type != UnitType::Zerg_Overlord)
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::{HaveSeen, UnitId};

    #[test]
    fn test_destroy_missing_id() {
        let mut seen = HaveSeen::new();
        let id: UnitId = 3023;
        seen.on_unit_destroy(id);
    }
}
