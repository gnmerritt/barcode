use crate::counts::Counts;
use crate::seen::HaveSeen;
use rsbwapi::{Game, Player, Unit, UnitType};
use std::collections::HashMap;

pub(crate) struct UnitComp {
    unit_counts: HashMap<UnitType, i32>,
    larva: Vec<Unit>,
}

// note: supply is doubled by BWAPI so that Zerglings can use an interger amount of supply
impl UnitComp {
    pub fn new(game: &Game) -> Self {
        UnitComp {
            unit_counts: game.self_().unwrap().get_units().iter().fold(
                HashMap::new(),
                |mut acc, u| {
                    let type_ = u.get_type();
                    let type_ = match type_ {
                        UnitType::Zerg_Larva | UnitType::Zerg_Egg if u.is_morphing() => {
                            u.get_build_type()
                        }
                        _ => type_,
                    };
                    // count finished buildings only
                    if !type_.is_building() || !u.is_morphing() {
                        let increment = if type_.is_two_units_in_one_egg() {
                            2
                        } else {
                            1
                        };
                        *acc.entry(type_).or_insert(0) += increment;
                    }
                    acc
                },
            ),
            larva: vec![],
        }
    }

    pub fn count_of(&self, u: UnitType) -> i32 {
        *self.unit_counts.get(&u).unwrap_or(&0)
    }

    fn spawn_maybe(&mut self, counts: &mut Counts, utype: UnitType) -> Option<UnitType> {
        if let Some(larva) = self.larva.pop() {
            let res = larva.train(utype);
            if let Ok(true) = res {
                counts.bought(utype);
                println!(
                    "frame {} :: spawning a {:?} at {}. bank m={} g={}",
                    counts.frame(),
                    utype,
                    counts.supply_string(),
                    counts.minerals(),
                    counts.gas(),
                );
                *self.unit_counts.entry(utype).or_insert(0) += 1;
                return Some(utype);
            } else {
                self.larva.push(larva);
                println!(
                    "frame {} :: failed to spawn {:?} -> {:?}",
                    counts.frame(),
                    utype,
                    res
                );
            }
        }
        None
    }

    fn get_next_unit(&self, counts: &Counts, self_: &Player) -> Option<UnitType> {
        let used = counts.supply_used();
        let max = counts.supply_max();

        // check overlords first: first one at 9, rest 2 supply early
        if (used <= 18 && used == max) || (used >= max - 4) {
            let morphing_overlords = self_
                .get_units()
                .iter()
                .filter(|u| {
                    (u.get_type() == UnitType::Zerg_Egg || u.get_type() == UnitType::Zerg_Larva)
                        && u.is_morphing()
                        && u.get_build_type() == UnitType::Zerg_Overlord
                })
                .count();
            let needed_overlords = if used < 60 { 1 } else { 2 };
            if morphing_overlords < needed_overlords && counts.can_afford(UnitType::Zerg_Overlord) {
                return Some(UnitType::Zerg_Overlord);
            }
        }

        if used == max {
            return None; // no more supply
        }

        let has_pool = self.count_of(UnitType::Zerg_Spawning_Pool) >= 1;
        let has_spire = self.count_of(UnitType::Zerg_Spire) >= 1;

        // TODO:
        // make declarative rather than list of spawn statements
        // ratio of attacking units + number of drones/hatches req to support

        if has_pool
            && self.count_of(UnitType::Zerg_Zergling) < 8
            && counts.can_afford(UnitType::Zerg_Zergling)
        {
            return Some(UnitType::Zerg_Zergling);
        }

        if self.count_of(UnitType::Zerg_Drone) < 18 && counts.can_afford(UnitType::Zerg_Drone) {
            return Some(UnitType::Zerg_Drone);
        }

        if has_spire && counts.can_afford(UnitType::Zerg_Mutalisk) {
            return Some(UnitType::Zerg_Mutalisk);
        }

        if self.count_of(UnitType::Zerg_Drone) < 60 && counts.can_afford(UnitType::Zerg_Drone) {
            return Some(UnitType::Zerg_Drone);
        }

        None
    }

    pub fn spawn_units(&mut self, game: &Game, counts: &mut Counts, _seen: &HaveSeen) {
        if let Some(self_) = game.self_() {
            self.larva = self_
                .get_units()
                .into_iter()
                .filter(|u| {
                    u.get_type() == UnitType::Zerg_Larva
                        && !u.is_morphing()
                        && u.can_command().unwrap_or(false)
                })
                .collect();

            while let Some(next) = self.get_next_unit(counts, &self_) {
                if self.spawn_maybe(counts, next).is_none() {
                    break;
                }
            }
        }
    }
}
