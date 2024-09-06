// first pass: a volley calculator that doesn't take into account many things including:
// high ground misses
// splash damage
// shield/hp regen

use rsbwapi::{DamageType, UnitSizeType, UnitType, WeaponType};

#[derive(PartialEq, Default, Debug)]
struct Damage {
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

// TODO: calculate armor and weapon upgrade damage using
// Player::damage(WeaponType) and Player::armor(UnitType) at struct creation

#[derive(Debug)]
struct SimWeapon {
    type_: WeaponType,
    upgrade_damage: f32,
}

#[derive(Debug)]
struct SimUnit {
    type_: UnitType,
    armor: f32,
    shield_armor: f32,
    hp: f32,      // hp rounds up
    shields: f32, // shields round down
}

fn get_damage_ratio(wpn: WeaponType, target_size: UnitSizeType) -> f32 {
    static DAMAGE_RATIO: [[f32; UnitSizeType::MAX as usize]; DamageType::MAX as usize] = [
        // Ind, Sml, Med, Lrg, Non, Unk
        [0.0; UnitSizeType::MAX as usize], // Independent
        [0.0, 0.5, 0.75, 1.0, 0.0, 0.0],   // Explosive
        [0.0, 1.0, 0.5, 0.25, 0.0, 0.0],   // Concussive
        [0.0, 1.0, 1.0, 1.0, 0.0, 0.0],    // Normal
        [0.0, 1.0, 1.0, 1.0, 0.0, 0.0],    // Ignore_Armor
        [0.0; UnitSizeType::MAX as usize], // None
        [0.0; UnitSizeType::MAX as usize], // Unknown
    ];
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
        let mut wep_damage = weapon.type_.damage_amount() as f32 + weapon.upgrade_damage;
        let shields_remaining = target.shields - shield_damage;

        // damage applied to shields first, no size ratio adjustment
        // shield rounds down so a full 1.0 of shield is required before it
        // factors into damage calculations
        let attack_shield_damage = if shields_remaining > 1.0 {
            wep_damage -= target.shield_armor;
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
            let attack_hp_damage = damage_ratio * (wep_damage - target.armor) as f32;
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

#[cfg(test)]
mod test {
    use super::*;

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
        let ling = SimWeapon {
            type_: WeaponType::Claws,
            upgrade_damage: 0.0,
        };
        let marine = SimUnit {
            type_: UnitType::Terran_Marine,
            shield_armor: 0.0,
            armor: 0.0,
            hp: 40.0,
            shields: 0.0,
        };
        assert_eq!(
            damage_per_hit(&ling, &marine),
            Damage::hp(5.0),
            "ling v marine"
        );
    }

    #[test]
    fn test_dph_shields_armor() {
        let hydra = SimWeapon {
            type_: WeaponType::Needle_Spines,
            upgrade_damage: 1.0,
        };
        let zealot = SimUnit {
            type_: UnitType::Protoss_Zealot,
            shield_armor: 0.0,
            armor: 1.0,
            hp: 100.0,
            shields: 8.0,
        };
        assert_eq!(
            damage_per_hit(&hydra, &zealot),
            Damage::new(1.0, 8.0),
            "+1 hydra v zealot"
        );
    }
}
