#![allow(unused)]
use rsbwapi::{
    unit::UnitOrPosition, Player, PlayerId, ScaledPosition, TechType, Unit, UnitId, UnitType,
    WeaponType,
};

mod engine;
mod volleys;

#[derive(Debug, Clone)]
pub(crate) enum TargetType {
    Ground,
    Air,
    // TODO: spell types
}

#[derive(Debug, Clone)]
pub(crate) struct SimWeapon {
    type_: WeaponType,
    targets: TargetType,
    upgrade_damage: i32,
    range_min: i32,
    range_max: i32,
    cooldown: i32,
}

impl SimWeapon {
    pub(crate) fn from_weapon(player: &Player, wep: &WeaponType) -> Self {
        SimWeapon {
            type_: wep.clone(),
            targets: SimWeapon::get_target_type(wep),
            upgrade_damage: wep.damage_bonus() * player.get_upgrade_level(wep.upgrade_type()),
            range_min: wep.min_range(),
            range_max: wep.max_range(), // TODO range upgrades
            cooldown: wep.damage_cooldown(),
        }
    }

    fn get_target_type(wep: &WeaponType) -> TargetType {
        if wep.targets_air() {
            TargetType::Air
        } else {
            TargetType::Ground
        }
    }

    #[cfg(test)]
    fn simple(wep: WeaponType, upgrade_damage: i32) -> Self {
        SimWeapon {
            type_: wep,
            targets: SimWeapon::get_target_type(&wep),
            upgrade_damage,
            range_min: wep.min_range(),
            range_max: wep.max_range(),
            cooldown: wep.damage_cooldown(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SimUnit {
    id: UnitId,
    player: PlayerId,
    type_: UnitType,
    last_attack_frame: u32,
    position: ScaledPosition<1>,
    size: ScaledPosition<1>,
    facing: f64, // in radians, 0.0 is east
    weapons: Vec<SimWeapon>,
    armor: i32,
    shield_armor: i32,
    hp: f32,      // hp rounds up
    shields: f32, // shields round down
}

impl SimUnit {
    pub(crate) fn from_unit(unit: &Unit) -> Self {
        let type_ = unit.get_type();
        SimUnit {
            type_,
            id: unit.get_id(),
            player: unit.get_player().get_id(),
            last_attack_frame: 0,
            position: unit.get_position(),
            facing: unit.get_angle(),
            size: (type_.width(), type_.height()).into(),
            weapons: SimUnit::get_weapons(&unit.get_player(), type_),
            hp: unit.get_hit_points() as f32,
            shields: unit.get_shields() as f32,
            armor: unit.get_armor(),
            shield_armor: unit.get_shield_armor(),
        }
    }

    fn get_weapons(player: &Player, type_: UnitType) -> Vec<SimWeapon> {
        [type_.air_weapon(), type_.ground_weapon()]
            .into_iter()
            .filter_map(|wt| match wt {
                WeaponType::None | WeaponType::Unknown => None,
                _ => Some(SimWeapon::from_weapon(player, &wt)),
            })
            .collect()
    }

    #[cfg(test)]
    fn simple(type_: UnitType, shield_armor: i32, armor: i32, hp: f32, shields: f32) -> Self {
        SimUnit {
            type_,
            shield_armor,
            armor,
            hp,
            shields,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
enum Order {
    Guard,
    Attack(UnitId),
    AttackMove(ScaledPosition<1>),
    Move(ScaledPosition<1>),
    Follow(UnitId),
    Repair(UnitId),
    Hold,
    Stop,
    Patrol(ScaledPosition<1>),
    GroundAbility(TechType, ScaledPosition<1>),
    TargettedAbility(TechType, UnitId),
}

#[derive(Debug)]
struct SimOrder {
    unit: UnitId,
    order: Order,
    issued_frame: i32,
}

impl SimOrder {
    fn new(unit: UnitId, order: Order, issued_frame: i32) -> Self {
        SimOrder {
            unit,
            order,
            issued_frame,
        }
    }
}
