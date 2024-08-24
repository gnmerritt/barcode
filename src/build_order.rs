use crate::counts::Counts;
use rsbwapi::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq)]
struct BuildStep {
    unit_type: UnitType,
    min_supply: i32,
    building_type_count: i8,
}

impl BuildStep {
    fn new(unit_type: UnitType, min_supply: i32, building_type_count: i8) -> Self {
        BuildStep {
            unit_type,
            min_supply,
            building_type_count,
        }
    }
}

#[derive(Debug)]
struct PlacedBuilding {
    placed_frame: i32,
    building_type: UnitType,
    builder: Option<Unit>,
}

pub struct BuildOrder {
    frame: i32,
    to_build: Vec<BuildStep>,
    building_counts: HashMap<UnitType, i8>,
    placed_buildings: Vec<PlacedBuilding>,
    building_ids: HashSet<usize>,
}

impl BuildOrder {
    pub fn new() -> Self {
        BuildOrder {
            frame: 0,
            to_build: vec![
                // NB: these supplies written like they'd show up in build orders
                BuildStep::new(UnitType::Zerg_Hatchery, 11, 2),
                BuildStep::new(UnitType::Zerg_Spawning_Pool, 10, 1),
                BuildStep::new(UnitType::Zerg_Extractor, 9, 1),
                BuildStep::new(UnitType::Zerg_Lair, 10, 1),
                BuildStep::new(UnitType::Zerg_Extractor, 9, 2),
                BuildStep::new(UnitType::Zerg_Spire, 14, 1),
                BuildStep::new(UnitType::Zerg_Hatchery, 30, 3),
                BuildStep::new(UnitType::Zerg_Hatchery, 50, 4),
                BuildStep::new(UnitType::Zerg_Hydralisk_Den, 40, 1),
                BuildStep::new(UnitType::Zerg_Extractor, 50, 3),
                BuildStep::new(UnitType::Zerg_Queens_Nest, 50, 1),
                BuildStep::new(UnitType::Zerg_Hive, 50, 1),
                BuildStep::new(UnitType::Zerg_Defiler_Mound, 50, 1),
                BuildStep::new(UnitType::Zerg_Hatchery, 60, 9),
            ],
            building_counts: HashMap::new(),
            placed_buildings: vec![],
            building_ids: HashSet::new(),
        }
    }

    pub fn on_frame(&mut self, game: &Game) {
        self.frame = game.get_frame_count();
        if let Some(self_) = game.self_() {
            let buildings = self_
                .get_units()
                .into_iter()
                .filter(|u| {
                    u.get_type().is_building()
                        || (u.is_morphing() && u.get_build_type().is_building())
                })
                .map(|u| {
                    if u.get_type().is_building() {
                        (u.get_id(), u.get_type())
                    } else {
                        (u.get_id(), u.get_build_type())
                    }
                })
                .collect();
            self.check_placed_buildings(buildings);
        }
    }

    pub fn get_next_building(&self, counts: &Counts) -> Option<UnitType> {
        let supply_used = counts.supply_used();
        for step in self.to_build.iter() {
            let count = self.building_counts.get(&step.unit_type).unwrap_or(&0);
            if *count < step.building_type_count {
                // remember that BW doubles supplies
                if supply_used >= 2 * step.min_supply {
                    return Some(step.unit_type.clone());
                } else if counts.minerals() > 1_000 {
                    return Some(UnitType::Zerg_Hatchery);
                } else {
                    return None;
                }
            }
        }
        None
    }

    /**
     * Keep track of buildings that have been placed but the drone may not have
     * started morphing yet
     */
    pub fn placed_building(&mut self, building_type: UnitType, builder: Option<Unit>) {
        self.placed_buildings.push(PlacedBuilding {
            building_type,
            builder,
            placed_frame: self.frame,
        });
        self.count_type(building_type);
    }

    pub fn upgraded_building(&mut self, building_type: UnitType) {
        self.placed_building(building_type, None)
    }

    fn count_type(&mut self, building_type: UnitType) {
        self.building_counts
            .entry(building_type)
            .and_modify(|c| *c += 1)
            .or_insert(1);
    }

    // remove buildings that have begun construction from our placed list
    // so we don't double-count their cost
    fn check_placed_buildings(&mut self, buildings: Vec<(UnitId, UnitType)>) {
        self.building_counts.clear();

        for (id, bt) in buildings {
            self.count_type(bt);

            // if this is the first frame they've existed make sure to remove
            // them from the placed buildings list
            if !self.building_ids.contains(&id) {
                println!(
                    "saw new building of {:?}, placed={:?}",
                    bt, self.placed_buildings
                );
                self.building_ids.insert(id);
                let index = self
                    .placed_buildings
                    .iter()
                    .position(|t| t.building_type == bt);
                if let Some(index) = index {
                    let pb = self.placed_buildings.swap_remove(index);
                    println!(
                        "{:?} started after {} frames",
                        pb,
                        self.frame - pb.placed_frame
                    );
                }
            }
        }

        // stop tracking placed builings after 150 frames
        // TODO replace with watching the drone's id
        //        self.placed_buildings
        //          .retain(|pb| pb.placed_frame + 150 > self.frame);

        let mut failed_to_build = vec![];
        for (i, pb) in self.placed_buildings.iter().enumerate() {
            let mut failed = false;
            if let Some(builder) = pb.builder.as_ref() {
                if !builder.exists() {
                    println!(
                        "frame {} :: {:?} failed to build, builder died",
                        self.frame, pb.building_type
                    );
                    failed = true;
                }
                // TODO: similar check for building upgrades if we see them fail?
                if self.frame > pb.placed_frame + 10 && builder.is_idle() {
                    println!(
                        "frame {} :: {:?} has failed to build after {} frames",
                        self.frame,
                        pb.building_type,
                        self.frame - pb.placed_frame
                    );
                    failed = true;
                }
            }

            if failed {
                failed_to_build.push(i);
                continue;
            }

            // count placed buildings in our builder too
            // replacing this with the method angers the borrow checker :-(
            self.building_counts
                .entry(pb.building_type)
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }
        for i in failed_to_build {
            self.placed_buildings.swap_remove(i);
        }
    }

    pub fn spent_minerals(&self) -> i32 {
        self.placed_buildings
            .iter()
            .map(|b| b.building_type.mineral_price())
            .sum()
    }

    pub fn spent_gas(&self) -> i32 {
        self.placed_buildings
            .iter()
            .map(|b| b.building_type.gas_price())
            .sum()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_building() {
        let mut bo = BuildOrder::new();
        // we start out with one hatch
        bo.check_placed_buildings(vec![(10, UnitType::Zerg_Hatchery)]);

        let c = Counts::new_fake(8);
        assert_eq!(bo.get_next_building(&c), None, "saw building too early");
        let c = Counts::new_fake(22);
        assert_eq!(
            bo.get_next_building(&c),
            Some(UnitType::Zerg_Hatchery),
            "got hatch first"
        );
        // no-op to place a building not in the order
        bo.placed_building(UnitType::Terran_Barracks, None);
        assert_eq!(
            bo.get_next_building(&c),
            Some(UnitType::Zerg_Hatchery),
            "still got hatch"
        );
        assert_eq!(
            bo.spent_minerals(),
            UnitType::Terran_Barracks.mineral_price(),
            "barracks mineral price accounted for if we say we placed it"
        );

        bo.placed_building(UnitType::Zerg_Hatchery, None);
        assert_eq!(
            bo.get_next_building(&c),
            Some(UnitType::Zerg_Spawning_Pool),
            "pool after hatch"
        );
    }

    #[test]
    fn test_spent_resources() {
        let mut bo = BuildOrder::new();
        bo.placed_building(UnitType::Zerg_Spire, None);
        assert_eq!(bo.spent_minerals(), UnitType::Zerg_Spire.mineral_price());
        assert_eq!(bo.spent_gas(), UnitType::Zerg_Spire.gas_price());
    }
}
