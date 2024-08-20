use crate::counts::Counts;
use crate::seen::HaveSeen;
use rsbwapi::{Game, Unit, UnitType};
use std::collections::HashMap;

pub(crate) struct UnitComp {
    unit_counts: HashMap<UnitType, i32>,
}

fn spawn_maybe(counts: &mut Counts, larva: Option<Unit>, utype: UnitType) -> Option<UnitType> {
    if let Some(larva) = larva {
        if let Ok(true) = larva.train(utype) {
            println!("spawning a {:?} at {}", utype, counts.supply_string());
            counts.bought(utype);
            return Some(utype);
        }
    }
    None
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
                        UnitType::Zerg_Larva if u.is_morphing() => u.get_build_type(),
                        _ => type_,
                    };
                    *acc.entry(type_).or_insert(0) += 1;
                    acc
                },
            ),
        }
    }

    pub fn count_of(&self, u: UnitType) -> i32 {
        *self.unit_counts.get(&u).unwrap_or(&0)
    }

    pub fn spawn_units(
        &self,
        game: &Game,
        counts: &mut Counts,
        _seen: &HaveSeen,
    ) -> Option<String> {
        if let Some(self_) = game.self_() {
            let used = counts.supply_used();
            let max = counts.supply_max();
            let mut larva = self_
                .get_units()
                .into_iter()
                .filter(|u| u.get_type() == UnitType::Zerg_Larva);

            // check overlords first: first one at 9, rest 2 supply early
            if (used <= 18 && used == max) || (used >= max - 4) {
                let morphing_overlords = self_
                    .get_units()
                    .iter()
                    .filter(|u| {
                        u.get_type() == UnitType::Zerg_Egg
                            && u.is_morphing()
                            && u.get_build_type() == UnitType::Zerg_Overlord
                    })
                    .count();
                let needed_overlords = if used < 60 { 1 } else { 2 };
                if morphing_overlords < needed_overlords
                    && counts.can_afford(UnitType::Zerg_Overlord)
                {
                    spawn_maybe(counts, larva.next(), UnitType::Zerg_Overlord);
                }
            }

            // TODO:
            // link units to tech structures
            // make declarative rather than list of spawn statements
            // ratio of attacking units + number of drones/hatches req to support

            if self.count_of(UnitType::Zerg_Zergling) < 8
                && counts.can_afford(UnitType::Zerg_Zergling)
            {
                spawn_maybe(counts, larva.next(), UnitType::Zerg_Zergling);
            }

            if self.count_of(UnitType::Zerg_Drone) < 24 && counts.can_afford(UnitType::Zerg_Drone) {
                spawn_maybe(counts, larva.next(), UnitType::Zerg_Drone);
            }

            if self.count_of(UnitType::Zerg_Zergling) < 16
                && counts.can_afford(UnitType::Zerg_Zergling)
            {
                spawn_maybe(counts, larva.next(), UnitType::Zerg_Zergling);
            }

            if counts.can_afford(UnitType::Zerg_Mutalisk) {
                spawn_maybe(counts, larva.next(), UnitType::Zerg_Mutalisk);
            }

            if self.count_of(UnitType::Zerg_Drone) < 60 && counts.can_afford(UnitType::Zerg_Drone) {
                spawn_maybe(counts, larva.next(), UnitType::Zerg_Drone);
            }
        }
        None
    }
}
