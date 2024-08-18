use rsbwapi::{Game, TilePosition, Unit, UnitId, UnitType};
use std::collections::{HashMap, HashSet};

struct SeenUnit {
    unit: Unit,
    frame: i32,
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

    pub fn on_frame(&mut self, game: &Game) {
        if let Some(self_) = game.self_() {
            let frame = game.get_frame_count();
            let not_ours = game
                .get_all_units()
                .into_iter()
                .filter(|u| self_ != u.get_player());
            let mut frame_counts = HashMap::new();
            for unit in not_ours {
                let type_ = unit.get_type();
                match type_ {
                    UnitType::Resource_Mineral_Field => {
                        self.minerals.insert(unit.get_tile_position().clone());
                    }
                    UnitType::Resource_Vespene_Geyser => {
                        self.gas_locs.insert(unit.get_tile_position().clone());
                    }
                    _ if type_.is_refinery() => {
                        self.gas_locs.insert(unit.get_tile_position().clone());
                        frame_counts
                            .entry(type_)
                            .and_modify(|c| *c += 1)
                            .or_insert(0);
                    }
                    _ if type_.is_building() => {
                        frame_counts
                            .entry(type_)
                            .and_modify(|c| *c += 1)
                            .or_insert(0);
                        self.buildings
                            .insert(unit.get_id(), SeenUnit { unit, frame });
                    }
                    _ => {
                        self.units.insert(unit.get_id(), SeenUnit { unit, frame });
                    }
                }
            }
            // update the saved counts iff we saw more of a unit type in this frame
            // than we have before.
            // TODO: do this better with unit ids?
            for (k, v) in frame_counts.into_iter() {
                self.building_counts
                    .entry(k)
                    .and_modify(|c| *c = std::cmp::max(*c, v))
                    .or_insert(v);
            }
        }
    }

    pub fn on_unit_destroy(&mut self, id: UnitId) {
        self.units.remove(&id);
        if let Some(b) = self.buildings.remove(&id) {
            self.building_counts
                .entry(b.unit.get_type())
                .and_modify(|c| *c -= 1);
        }
    }

    pub fn get_gas_locs(&self) -> Vec<&TilePosition> {
        self.gas_locs.iter().collect()
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
