use super::{orders::OrderProcessor, volleys::Damage, Order, SimOrder, SimUnit};
use rsbwapi::{DamageType, Race, ScaledPosition, TilePosition, UnitId};
use std::collections::{HashMap, HashSet};

const ZERG_HP_REGEN: f32 = 4.0 / 256.0;
const TOSS_SHIELD_REGEN: f32 = 7.0 / 256.0;
const TERRAN_BURN: f32 = 20.0 / 256.0;
const ENV: UnitId = UnitId::MAX;

#[derive(Debug, PartialEq)]
pub(super) enum Effect {
    Idle(UnitId),
    Damaged(UnitId, Damage, UnitId),
    Healed(UnitId, Damage),
    Moved(UnitId, ScaledPosition<1>, f64),
    Died(UnitId),
}

/// Unit combat simulator
pub(super) struct Engagement {
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

    pub fn get_frame(&self) -> i32 {
        self.frame
    }

    pub fn all_units(&self) -> impl Iterator<Item = &SimUnit> {
        self.units.values()
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
        let processor = OrderProcessor::new(self, unit);
        let order_type = order.map(|o| o.order).unwrap_or(Order::Guard);
        match order_type {
            Order::Attack(target_id) => processor.attack_unit(self.units.get(&target_id)),
            Order::AttackMove(dest) => processor.attack_move(dest),
            Order::Follow(target_id) => processor.move_towards_unit(self.units.get(&target_id)),
            Order::Move(dest) => processor.move_towards(dest),
            Order::Patrol(from, to) => processor.attack_move(to), // TODO
            Order::Guard => processor.guard(),
            Order::Hold => processor.hold_position(),
            _ => Effect::Idle(unit.id),
        }
    }

    fn capped_heal(hp: &mut f32, heal: f32, max: i32) {
        *hp += heal;
        if *hp > max as f32 {
            *hp = max as f32;
        }
    }

    fn apply_effects(&mut self, effects: Vec<Effect>) -> FrameSummary {
        let mut summary = FrameSummary::new(self.frame);
        for e in effects {
            match e {
                Effect::Healed(id, ref heal) => {
                    if let Some(unit) = self.units.get_mut(&id) {
                        Engagement::capped_heal(&mut unit.hp, heal.hp, unit.type_.max_hit_points());
                        Engagement::capped_heal(
                            &mut unit.shields,
                            heal.shield,
                            unit.type_.max_shields(),
                        );
                    }
                }
                Effect::Damaged(attacker, ref damage, target) => {
                    if let Some(target) = self.units.get_mut(&target) {
                        target.hp -= damage.hp;
                        target.shields -= damage.shield;
                    }
                    if let Some(attacker) = self.units.get_mut(&attacker) {
                        attacker.last_attack_frame = self.frame;
                    }
                    summary.add(target, e);
                }
                Effect::Died(id) => {
                    self.deadpool.insert(id);
                    self.units.remove(&id);
                    summary.add(id, e);
                }
                Effect::Moved(id, new_pos, new_facing) => {
                    if let Some(unit) = self.units.get_mut(&id) {
                        unit.position = new_pos;
                        unit.facing = new_facing;
                    }
                }
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
                    Some(Effect::Damaged(ENV, Damage::hp(TERRAN_BURN), unit.id))
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
pub struct FrameSummary {
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
        for (id, effects) in summary.effects {
            for e in effects {
                self.add(id, e);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Engagement, TOSS_SHIELD_REGEN, ZERG_HP_REGEN};
    use crate::sim::{
        engine::{Effect, FrameSummary, ENV, TERRAN_BURN},
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
        assert_eq!(burning[0], Effect::Damaged(ENV, Damage::hp(TERRAN_BURN), 2));

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

        for _ in 0..3_000 {
            e.tick();
            let pylon = e.units.get(&2).unwrap();
            assert!(
                pylon.shields <= pylon.type_.max_shields() as f32,
                "shield overheal"
            )
        }
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
        for _ in 0..3_000 {
            e.tick();
            let sunken = e.units.get(&0).unwrap();
            assert!(
                sunken.hp <= sunken.type_.max_hit_points() as f32,
                "regen overheal"
            )
        }
    }
}
