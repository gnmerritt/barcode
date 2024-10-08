use crate::{counts::Counts, drones::DroneManager};
use rsbwapi::*;
use std::collections::{HashMap, HashSet};

pub(crate) trait TechChecker {
    fn has_prereqs(&self, unit_type: &UnitType) -> bool;
}

impl TechChecker for &Game {
    fn has_prereqs(&self, unit_type: &UnitType) -> bool {
        if let Some(self_) = self.self_() {
            unit_type
                .required_units()
                .into_iter()
                .all(|(unit, amount)| self_.has_unit_type_requirement(*unit, *amount))
        } else {
            false
        }
    }
}

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
    stuck_drones: Vec<UnitId>,
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
                BuildStep::new(UnitType::Zerg_Lair, 15, 1),
                BuildStep::new(UnitType::Zerg_Extractor, 11, 2),
                BuildStep::new(UnitType::Zerg_Spire, 15, 1),
                BuildStep::new(UnitType::Zerg_Hatchery, 30, 3),
                BuildStep::new(UnitType::Zerg_Hatchery, 50, 4),
                BuildStep::new(UnitType::Zerg_Hydralisk_Den, 40, 1),
                BuildStep::new(UnitType::Zerg_Extractor, 50, 3),
                BuildStep::new(UnitType::Zerg_Queens_Nest, 50, 1),
                BuildStep::new(UnitType::Zerg_Hive, 50, 1), // TODO we morph another lair when this starts
                BuildStep::new(UnitType::Zerg_Defiler_Mound, 50, 1),
                BuildStep::new(UnitType::Zerg_Hatchery, 60, 9),
            ],
            building_counts: HashMap::new(),
            placed_buildings: vec![],
            building_ids: HashSet::new(),
            stuck_drones: vec![],
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

    pub fn release_drones(&mut self, drones: &mut DroneManager) {
        for id in self.stuck_drones.iter() {
            drones.idle(*id);
        }
        self.stuck_drones.clear();
    }

    pub fn get_next_building(&self, tech: impl TechChecker, counts: &Counts) -> Option<UnitType> {
        let supply_used = counts.supply_used();
        for step in self.to_build.iter() {
            let count = self.building_counts.get(&step.unit_type).unwrap_or(&0);
            if *count < step.building_type_count {
                // remember that BW doubles supplies
                if supply_used >= 2 * step.min_supply {
                    if !tech.has_prereqs(&step.unit_type) {
                        return None;
                    }
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

    pub fn upgraded_building(&mut self, building: Unit, building_type: UnitType) {
        self.placed_building(building_type, Some(building));
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
            if bt == UnitType::Zerg_Lair || bt == UnitType::Zerg_Hive {
                self.count_type(UnitType::Zerg_Hatchery);
            }

            // if this is the first frame they've existed make sure to remove
            // them from the placed buildings list
            // NB: we check for building upgrades in the next pass below
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

        self.placed_buildings.retain(|pb| {
            if let Some(builder) = pb.builder.as_ref() {
                if !builder.exists() {
                    println!(
                        "frame {} :: {:?} failed to build, builder died",
                        self.frame, pb.building_type
                    );
                    return false;
                }
                // TODO: similar check for building upgrades if we see them fail?
                if self.frame > pb.placed_frame + 10 && builder.is_idle() {
                    println!(
                        "frame {} :: {:?} has failed to build after {} frames",
                        self.frame,
                        pb.building_type,
                        self.frame - pb.placed_frame
                    );
                    self.stuck_drones.push(builder.get_id());
                    return false;
                }
                if builder.get_type().is_building()
                    && builder.is_morphing()
                    && pb.building_type == builder.get_build_type()
                {
                    println!(
                        "frame {} :: {:?} upgrade started after {} frames",
                        self.frame,
                        pb,
                        self.frame - pb.placed_frame
                    );
                    // it didn't fail but it isn't "placed" once it's started
                    return false;
                }
            }
            true
        });

        // count placed buildings in our builder too
        // replacing this with the method angers the borrow checker :-(
        for pb in self.placed_buildings.iter() {
            self.building_counts
                .entry(pb.building_type)
                .and_modify(|c| *c += 1)
                .or_insert(1);
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

    struct AllTech;
    impl TechChecker for &AllTech {
        fn has_prereqs(&self, _unit_type: &UnitType) -> bool {
            true
        }
    }

    #[test]
    fn test_get_building() {
        let tech = AllTech {};
        let mut bo = BuildOrder::new();
        // we start out with one hatch
        bo.check_placed_buildings(vec![(10, UnitType::Zerg_Hatchery)]);

        let c = Counts::new_fake(8);
        assert_eq!(
            bo.get_next_building(&tech, &c),
            None,
            "saw building too early"
        );
        let c = Counts::new_fake(22);
        assert_eq!(
            bo.get_next_building(&tech, &c),
            Some(UnitType::Zerg_Hatchery),
            "got hatch first"
        );
        // no-op to place a building not in the order
        bo.placed_building(UnitType::Terran_Barracks, None);
        assert_eq!(
            bo.get_next_building(&tech, &c),
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
            bo.get_next_building(&tech, &c),
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
