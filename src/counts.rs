use crate::build_order::BuildOrder;
use rsbwapi::{Game, UnitType};

/**
 * Keep track of quantities more reliably than the game engine
 */
#[derive(Default)]
pub(crate) struct Counts {
    minerals: i32,
    gas: i32,
    supply_used: i32,
    supply_max: i32,
    frame: i32,
}

impl Counts {
    pub fn new(game: &Game, build_order: &BuildOrder) -> Self {
        use std::cmp::max;
        let mut c = Counts {
            ..Default::default()
        };
        c.frame = game.get_frame_count();
        if let Some(self_) = game.self_() {
            c.minerals = self_.minerals() - build_order.spent_minerals();
            c.gas = self_.gas() - build_order.spent_gas();
            c.supply_max = max(
                self_.supply_total(),
                self_
                    .get_units()
                    .iter()
                    .map(|u| u.get_type())
                    .filter(|t| {
                        *t == UnitType::Zerg_Overlord
                            || *t == UnitType::Zerg_Hatchery
                            || *t == UnitType::Zerg_Lair
                            || *t == UnitType::Zerg_Hive
                    })
                    .map(|ut| ut.supply_provided())
                    .sum(),
            );
            c.supply_used = self_.supply_used();
        }
        c
    }

    pub fn supply_string(&self) -> String {
        format!("supply {} / {}", self.supply_used, self.supply_max)
    }

    pub fn bought(&mut self, unit: UnitType) {
        self.minerals -= unit.mineral_price();
        self.gas -= unit.gas_price();
    }

    pub fn spent(&mut self, minerals: i32, gas: i32) {
        self.minerals -= minerals;
        self.gas -= gas;
    }

    pub fn can_afford(&self, unit: UnitType) -> bool {
        return self.minerals() >= unit.mineral_price() && self.gas() >= unit.gas_price();
    }

    pub fn minerals(&self) -> i32 {
        self.minerals
    }
    pub fn gas(&self) -> i32 {
        self.gas
    }
    pub fn supply_max(&self) -> i32 {
        self.supply_max
    }
    pub fn frame(&self) -> i32 {
        self.frame
    }
}
