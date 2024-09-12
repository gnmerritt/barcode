use super::{
    engine::{Effect, Engagement},
    volleys::{self, damage_per_hit, Damage},
    SimUnit,
};
use rsbwapi::ScaledPosition;

/// Order processing logic
pub(super) struct OrderProcessor<'a> {
    engagement: &'a Engagement,
    unit: &'a SimUnit,
}

impl<'a> OrderProcessor<'a> {
    pub fn new(engagement: &'a Engagement, unit: &'a SimUnit) -> Self {
        OrderProcessor { engagement, unit }
    }

    pub fn move_towards(&self, dest: ScaledPosition<1>) -> Effect {
        // TODO: use velocity+accel, facing direction, consider pathing/tiles, collisions
        let velocity = self.unit.type_.top_speed();
        let dist = self.unit.position.distance(dest);
        if dist <= velocity {
            Effect::Moved(self.unit.id, dest, self.unit.facing)
        } else {
            let progress: ScaledPosition<1> =
                position_towards(&self.unit.position, &dest, velocity);
            Effect::Moved(self.unit.id, progress, self.unit.facing)
        }
    }

    pub fn move_towards_unit(&self, dest: Option<&SimUnit>) -> Effect {
        if let Some(dest) = dest {
            self.move_towards(dest.position)
        } else {
            Effect::Idle(self.unit.id)
        }
    }

    pub fn attack_unit(&self, unit: Option<&SimUnit>) -> Effect {
        // TODO: hoist this out into a function with failure reasons
        // so we aren't repeating this mess everywhere
        if let Some(target) = unit {
            let wep = self.unit.weapons.iter().next(); // TODO
            if let Some(wep) = wep {
                let dist = self.unit.position.distance(target.position);
                let range_ok = dist <= wep.range_max && dist >= wep.range_min;
                if range_ok {
                    let cooldown_ok =
                        self.unit.last_attack_frame + wep.cooldown >= self.engagement.get_frame();
                    if cooldown_ok {
                        let damage = damage_per_hit(wep, target);
                        return Effect::Damaged(self.unit.id, damage, target.id);
                    }
                } else {
                    return self.move_towards_unit(unit);
                }
            }
        }
        return Effect::Idle(self.unit.id);
    }

    fn attack_anything_maybe(&self) -> Option<Effect> {
        // TODO make this do less work for e.g. when attacks in cooldown
        for target in self.engagement.all_units() {
            if self.unit.player == target.player {
                continue;
            }
            let attacked = self.attack_unit(Some(target));
            if let Effect::Damaged(_, _, _) = attacked {
                return Some(attacked);
            }
        }
        None
    }

    pub fn attack_move(&self, dest: ScaledPosition<1>) -> Effect {
        if let Some(attack) = self.attack_anything_maybe() {
            attack
        } else {
            self.move_towards(dest)
        }
    }

    pub fn hold_position(&self) -> Effect {
        if let Some(attack) = self.attack_anything_maybe() {
            attack
        } else {
            Effect::Idle(self.unit.id)
        }
    }

    pub fn guard(&self) -> Effect {
        if let Some(attack) = self.attack_anything_maybe() {
            attack
        } else {
            // TODO: move towards an enemy in aggro range
            Effect::Idle(self.unit.id)
        }
    }
}

fn position_towards(
    from: &ScaledPosition<1>,
    towards: &ScaledPosition<1>,
    velocity: f64,
) -> ScaledPosition<1> {
    let dist = from.distance(*towards);
    if dist == 0.0 || dist < velocity {
        return towards.clone();
    }
    let t = velocity / dist;
    let x = (1.0 - t) * from.x as f64 + t * towards.x as f64;
    let y = (1.0 - t) * from.y as f64 + t * towards.y as f64;
    ScaledPosition {
        x: x as i32,
        y: y as i32,
    }
}

#[cfg(test)]
mod test {
    use super::position_towards;
    use rsbwapi::ScaledPosition;

    #[test]
    fn test_position_towards() {
        let origin: ScaledPosition<1> = (0, 0).into();
        let dest: ScaledPosition<1> = (10, 10).into();
        let towards = position_towards(&origin, &dest, 4.25); // sqrt(18) ~= 4.25
        assert_eq!(towards.x, 3, "x right");
        assert_eq!(towards.y, 3, "y right");
    }
}
