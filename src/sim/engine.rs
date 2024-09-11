use super::{volleys::Damage, Order, SimOrder, SimUnit};
use rsbwapi::{DamageType, Race, ScaledPosition, TilePosition, UnitId};
use std::collections::{HashMap, HashSet};

const ZERG_HP_REGEN: f32 = 4.0 / 256.0;
const TOSS_SHIELD_REGEN: f32 = 7.0 / 256.0;
const TERRAN_BURN: f32 = 20.0 / 256.0;

#[derive(Debug, PartialEq)]
enum Effect {
    Idle(UnitId),
    Damaged(UnitId, Damage),
    Healed(UnitId, Damage),
    Moved(UnitId, ScaledPosition<1>, f64),
    Died(UnitId),
}

/// Unit combat simulator
struct Engagement {
    top_left: TilePosition,
    size: TilePosition,
    frame: i32,
    units: HashMap<UnitId, SimUnit>,
    orders: HashMap<UnitId, SimOrder>,
    deadpool: HashSet<UnitId>,
}

impl Engagement {
    pub fn new(
        frame: i32,
        top_left: TilePosition,
        size: TilePosition,
        units: Vec<SimUnit>,
    ) -> Self {
        Engagement {
            top_left,
            size,
            frame,
            units: units.into_iter().map(|u| (u.id, u)).collect(),
            orders: HashMap::new(),
            deadpool: HashSet::new(),
        }
    }

    #[cfg(test)]
    fn simple(units: Vec<SimUnit>) -> Self {
        Engagement::new(0, (0, 0).into(), (10, 10).into(), units)
    }

    pub fn tick(&mut self) -> FrameSummary {
        let mut effects = self.process_orders();
        effects.extend(self.burning());
        effects.extend(self.hp_shield_regen());
        // TODO: spells etc should fit here nicely
        let mut summary = self.apply_effects(effects);

        // take a second pass to clean up any units that died (hp < 0) this frame
        let died = self.mark_dead_units();
        let died = self.apply_effects(died);
        summary.extend(died);

        self.frame += 1;
        summary
    }

    fn process_orders(&self) -> Vec<Effect> {
        self.units
            .iter()
            .map(|(id, unit)| {
                let order = self.orders.get(&id);
                self.process_order(unit, order)
            })
            .collect()
    }

    fn process_order(&self, unit: &SimUnit, order: Option<&SimOrder>) -> Effect {
        // TODO: turn orders into effects
        Effect::Idle(unit.id)
    }

    fn apply_effects(&mut self, effects: Vec<Effect>) -> FrameSummary {
        let mut summary = FrameSummary::new(self.frame);
        for e in effects {
            match e {
                Effect::Healed(id, ref heal) => {
                    if let Some(unit) = self.units.get_mut(&id) {
                        unit.hp += heal.hp;
                        unit.shields += heal.shield;
                    }
                }
                Effect::Damaged(id, ref damage) => {
                    if let Some(unit) = self.units.get_mut(&id) {
                        unit.hp -= damage.hp;
                        unit.shields -= damage.shield;
                    }
                    summary.add(id, e);
                }
                Effect::Died(id) => {
                    self.deadpool.insert(id);
                    self.units.remove(&id);
                    summary.add(id, e);
                }
                Effect::Moved(id, new_pos, new_facing) => {}
                Effect::Idle(id) => {}
            }
        }
        summary
    }

    fn mark_dead_units(&self) -> Vec<Effect> {
        self.units
            .values()
            .filter_map(|unit| {
                if unit.hp < 0.0 {
                    Some(Effect::Died(unit.id))
                } else {
                    None
                }
            })
            .collect()
    }

    fn burning(&self) -> Vec<Effect> {
        self.units
            .values()
            .filter(|u| u.type_.get_race() == Race::Terran && u.type_.is_building())
            .filter_map(|unit| {
                let starts_burning = unit.type_.max_hit_points() as f32 / 3.0;
                if unit.hp <= starts_burning {
                    Some(Effect::Damaged(unit.id, Damage::hp(TERRAN_BURN)))
                } else {
                    None
                }
            })
            .collect()
    }

    fn hp_shield_regen(&self) -> Vec<Effect> {
        let mut shields: Vec<Effect> = self
            .units
            .values()
            .filter(|u| u.type_.get_race() == Race::Protoss)
            .filter_map(|unit| {
                let max_shields = unit.type_.max_shields() as f32;
                if unit.shields == max_shields {
                    None
                } else {
                    Some(Effect::Healed(unit.id, Damage::shield(TOSS_SHIELD_REGEN)))
                }
            })
            .collect();
        let hp = self
            .units
            .values()
            .filter(|u| u.type_.get_race() == Race::Zerg)
            .filter_map(|unit| {
                let max_hp = unit.type_.max_hit_points() as f32;
                if unit.hp == max_hp {
                    None
                } else {
                    Some(Effect::Healed(unit.id, Damage::hp(ZERG_HP_REGEN)))
                }
            });
        shields.extend(hp);
        shields
    }
}

#[derive(Debug)]
struct FrameSummary {
    frame: i32,
    effects: HashMap<UnitId, Vec<Effect>>,
}

impl FrameSummary {
    fn new(frame: i32) -> Self {
        FrameSummary {
            frame,
            effects: HashMap::new(),
        }
    }

    fn add(&mut self, unit: UnitId, effect: Effect) {
        let e = self.effects.entry(unit).or_insert_with(Vec::new);
        e.push(effect);
    }

    fn extend(&mut self, summary: FrameSummary) {
        for (id, effects) in summary.effects.into_iter() {
            for e in effects.into_iter() {
                self.add(id, e);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Engagement, TOSS_SHIELD_REGEN, ZERG_HP_REGEN};
    use crate::sim::{
        engine::{Effect, FrameSummary, TERRAN_BURN},
        volleys::Damage,
        SimUnit,
    };
    use rsbwapi::UnitType;

    #[test]
    fn test_burning() {
        let mut rax = SimUnit::simple(UnitType::Terran_Barracks, 0, 0, 0.45, 0.0);
        rax.id = 2;
        let units = vec![
            SimUnit::simple(UnitType::Terran_Command_Center, 0, 0, 1500.0, 0.0),
            rax,
            SimUnit::simple(UnitType::Protoss_Pylon, 0, 0, 10.0, 0.0),
        ];
        let mut e = Engagement::simple(units);

        let burning = e.burning();
        assert_eq!(burning.len(), 1, "only one building burned");
        // rax id == 2
        assert_eq!(burning[0], Effect::Damaged(2, Damage::hp(TERRAN_BURN)));

        // let the building burn down
        let mut summary = FrameSummary::new(0);
        for _ in 0..6 {
            let t = e.tick();
            summary.extend(t);
        }

        assert_eq!(e.deadpool.len(), 1, "one unit has died");
        assert!(e.deadpool.contains(&2), "rax died");

        let rax_effects = summary.effects.get(&2).expect("no effects on the rax");
        assert_eq!(rax_effects.len(), 7, "burned for 6 frames and then died");
        assert_eq!(rax_effects[6], Effect::Died(2), "rax marked dead correctly");
    }

    #[test]
    fn test_shields() {
        let mut pylon = SimUnit::simple(UnitType::Protoss_Pylon, 0, 0, 10.0, 0.0);
        pylon.id = 2;
        let units = vec![
            SimUnit::simple(UnitType::Terran_Command_Center, 0, 0, 1500.0, 0.0),
            pylon,
        ];
        let mut e = Engagement::simple(units);
        let shields = e.hp_shield_regen();
        assert_eq!(shields.len(), 1, "one shield regen building");
        assert_eq!(
            shields[0],
            Effect::Healed(2, Damage::shield(TOSS_SHIELD_REGEN))
        );
    }

    #[test]
    fn test_regen() {
        let units = vec![SimUnit::simple(
            UnitType::Zerg_Sunken_Colony,
            0,
            0,
            10.0,
            0.0,
        )];
        let mut e = Engagement::simple(units);
        let regen = e.hp_shield_regen();
        assert_eq!(regen.len(), 1, "zerg building regenerated");
        assert_eq!(regen[0], Effect::Healed(0, Damage::hp(ZERG_HP_REGEN)));
    }
}
