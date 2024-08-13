use rsbwapi::*;

pub fn position_building(game: &Game, bt: UnitType, builder: &Unit) -> Option<(i32, i32)> {
    match bt {
        UnitType::Zerg_Hatchery => position_new_base(game, bt, builder),
        _ if bt.is_building() => position_anywhere(game, bt, builder),
        _ => None,
    }
}

fn position_new_base(game: &Game, bt: UnitType, builder: &Unit) -> Option<(i32, i32)> {
    position_anywhere(game, bt, builder) // TODO!
}

fn position_anywhere(game: &Game, bt: UnitType, builder: &Unit) -> Option<(i32, i32)> {
    for y in 0..game.map_height() {
        for x in 0..game.map_width() {
            if game
                .can_build_here(builder, (x, y), bt, true)
                .unwrap_or(false)
            {
                return Some((x, y));
            }
        }
    }
    None
}
