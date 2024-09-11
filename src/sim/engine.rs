use std::collections::{HashMap, HashSet};

use rsbwapi::{Race, TilePosition, UnitId};

use super::{SimOrder, SimUnit};

const ZERG_HP_REGEN: f32 = 4.0 / 256.0;
const TOSS_SHIELD_REGEN: f32 = 7.0 / 256.0;
const TERRAN_BURN: f32 = 20.0 / 256.0;

/// Unit combat simulator
struct Engagement {
    top_left: TilePosition,
    size: TilePosition,
    frame: u32,
    units: Vec<SimUnit>,
    orders: HashMap<UnitId, SimOrder>,
    deadpool: HashSet<UnitId>,
}

impl Engagement {
    fn new() -> Self {
        unimplemented!()
    }

    fn tick(&mut self) -> Option<FrameSummary> {
        self.frame += 1;
        self.process_orders();
        self.burning();
        self.cull_dead_units();
        self.hp_shield_regen();
        None
    }

    fn process_orders(&mut self) {}

    fn cull_dead_units(&mut self) {
        self.units.retain(|unit| {
            if unit.hp < 0.0 {
                self.orders.remove(&unit.id);
                self.deadpool.insert(unit.id);
                false
            } else {
                true
            }
        });
    }

    fn burning(&mut self) {
        self.units
            .iter_mut()
            .filter(|u| u.type_.get_race() == Race::Terran && u.type_.is_building())
            .for_each(|unit| {
                let starts_burning = unit.type_.max_hit_points() as f32 / 3.0;
                if unit.hp <= starts_burning {
                    unit.hp -= TERRAN_BURN;
                }
            });
    }

    fn hp_shield_regen(&mut self) {
        self.units
            .iter_mut()
            .filter(|u| u.type_.get_race() == Race::Protoss)
            .for_each(|unit| {
                unit.shields += TOSS_SHIELD_REGEN;
                let max_shields = unit.type_.max_shields() as f32;
                if unit.shields > max_shields {
                    unit.shields = max_shields;
                }
            });
        self.units
            .iter_mut()
            .filter(|u| u.type_.get_race() == Race::Zerg)
            .for_each(|unit| {
                unit.hp += ZERG_HP_REGEN;
                let max_hp = unit.type_.max_hit_points() as f32;
                if unit.hp > max_hp {
                    unit.hp = max_hp;
                }
            });
    }
}

struct FrameSummary {}
