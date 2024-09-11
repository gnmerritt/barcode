// first pass: a volley calculator that doesn't take into account many things including:
// high ground misses
// splash damage
// shield/hp regen

use super::{SimUnit, SimWeapon};
use rsbwapi::{DamageType, UnitSizeType, WeaponType};

#[derive(PartialEq, Default, Debug)]
pub(crate) struct Damage {
    shield: f32,
    hp: f32,
}

impl Damage {
    fn hp(hp: f32) -> Self {
        Damage {
            hp,
            ..Default::default()
        }
    }

    fn new(hp: f32, shield: f32) -> Self {
        Damage { hp, shield }
    }
}

const DAMAGE_RATIO: [[f32; UnitSizeType::MAX as usize]; DamageType::MAX as usize] = [
    // Ind, Sml, Med, Lrg, Non, Unk
    [0.0; UnitSizeType::MAX as usize], // Independent
    [0.0, 0.5, 0.75, 1.0, 0.0, 0.0],   // Explosive
    [0.0, 1.0, 0.5, 0.25, 0.0, 0.0],   // Concussive
    [0.0, 1.0, 1.0, 1.0, 0.0, 0.0],    // Normal
    [0.0, 1.0, 1.0, 1.0, 0.0, 0.0],    // Ignore_Armor
    [0.0; UnitSizeType::MAX as usize], // None
    [0.0; UnitSizeType::MAX as usize], // Unknown
];

fn get_damage_ratio(wpn: WeaponType, target_size: UnitSizeType) -> f32 {
    DAMAGE_RATIO[wpn.damage_type() as usize][target_size as usize]
}

const MIN_DAMAGE: f32 = 0.5;

// Damage per attack animation from a weapon to a target
// references:
// https://bwcalc.com/
// https://liquipedia.net/starcraft/Damage_Order_of_Operations
fn damage_per_hit(weapon: &SimWeapon, target: &SimUnit) -> Damage {
    let mut shield_damage = 0.0;
    let mut hp_damage = 0.0;

    let damage_ratio = get_damage_ratio(weapon.type_, target.type_.size());

    for _ in 0..weapon.type_.damage_factor() {
        let mut wep_damage = (weapon.type_.damage_amount() + weapon.upgrade_damage) as f32;
        let shields_remaining = target.shields - shield_damage;

        // damage applied to shields first, no size ratio adjustment
        // shield rounds down so a full 1.0 of shield is required before it
        // factors into damage calculations
        let attack_shield_damage = if shields_remaining > 1.0 {
            wep_damage -= target.shield_armor as f32;
            let dealt_to_shield = if shields_remaining > wep_damage {
                wep_damage
            } else {
                shields_remaining
            };
            wep_damage -= dealt_to_shield;
            dealt_to_shield
        } else {
            0.0
        };
        shield_damage += attack_shield_damage;

        // if there is remaining damage it is applied to hp
        if wep_damage > 0.0 {
            let attack_hp_damage = damage_ratio * (wep_damage - target.armor as f32);
            if attack_hp_damage < MIN_DAMAGE && attack_shield_damage == 0.0 {
                hp_damage += MIN_DAMAGE;
            } else {
                hp_damage += attack_hp_damage;
            }
        }
    }

    Damage {
        shield: shield_damage,
        hp: hp_damage,
    }
}

/// returns the number of volleys with a weapon required to kill a unit
/// doesn't factor in timing or legal targetability (air/ground)
pub(crate) fn volleys_to_kill(target: &SimUnit, weapon: &SimWeapon) -> i32 {
    let mut target = target.clone();
    let mut volleys = 0;
    while target.hp > 0.0 {
        let Damage { shield, hp } = damage_per_hit(weapon, &target);
        target.shields -= shield;
        target.hp -= hp;
        volleys += 1;
    }
    volleys
}

#[cfg(test)]
mod test {
    use super::*;
    use rsbwapi::UnitType;

    #[test]
    fn test_damage_ratio() {
        assert_eq!(
            get_damage_ratio(WeaponType::Phase_Disruptor, UnitSizeType::Small),
            0.5,
            "dragoon -> zergling deals half"
        );
        assert_eq!(
            get_damage_ratio(WeaponType::Needle_Spines, UnitSizeType::Large),
            1.0,
            "hydra deals full against large"
        );
        assert_eq!(
            get_damage_ratio(WeaponType::Fragmentation_Grenade, UnitSizeType::Large),
            0.25,
            "vultures so bad against tanks"
        );
    }

    #[test]
    fn test_damage_per_hit() {
        let ling = SimWeapon::simple(WeaponType::Claws, 0);
        let marine = SimUnit::simple(UnitType::Terran_Marine, 0, 0, 40.0, 0.0);
        assert_eq!(
            damage_per_hit(&ling, &marine),
            Damage::hp(5.0),
            "ling v marine"
        );
        assert_eq!(volleys_to_kill(&marine, &ling), 8, "ling v marine");
    }

    #[test]
    fn test_dph_shields_armor() {
        let hydra = SimWeapon::simple(WeaponType::Needle_Spines, 1);
        let zealot = SimUnit::simple(UnitType::Protoss_Zealot, 0, 1, 100.0, 8.0);
        assert_eq!(
            damage_per_hit(&hydra, &zealot),
            Damage::new(1.0, 8.0),
            "+1 hydra v zealot"
        );
        let mut full_zealot = zealot.clone();
        full_zealot.shields = 60.0;
        assert_eq!(
            volleys_to_kill(&full_zealot, &hydra),
            26, // per bwcalc when you have more than 1 hydra attacking
            "+1 hydra v zealot"
        );
    }

    #[test]
    fn test_some_damage_factors() {
        assert_eq!(
            WeaponType::Hellfire_Missile_Pack.damage_factor(),
            2,
            "hellfire missles come in twos"
        );
        assert_eq!(
            WeaponType::Psi_Blades.damage_factor(),
            2,
            "two zealot hits at a time"
        );
    }

    #[test]
    fn test_dph_damage_factor() {
        let zealot = SimWeapon::simple(WeaponType::Psi_Blades, 2);
        let ling = SimUnit::simple(UnitType::Zerg_Zergling, 0, 1, 35.0, 0.0);
        assert_eq!(
            damage_per_hit(&zealot, &ling),
            Damage::hp(18.0),
            "+2 zealot v +1 armor ling"
        );
        assert_eq!(
            volleys_to_kill(&ling, &zealot),
            2,
            "+1 net zealot two-shots a ling"
        );
    }
}
