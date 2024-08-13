use rsbwapi::*;

pub(crate) struct BuildLoc {
    pub x: i32,
    pub y: i32,
}

pub fn position_building(game: &Game, bt: UnitType, builder: &Unit) -> Option<BuildLoc> {
    match bt {
        UnitType::Zerg_Hatchery => position_new_base(game, builder),
        _ if bt.is_building() => position_anywhere(game, bt, builder),
        _ => None,
    }
}

fn position_new_base(game: &Game, builder: &Unit) -> Option<BuildLoc> {
    let hatches: Vec<_> = game
        .self_()
        .unwrap()
        .get_units()
        .into_iter()
        .filter(|u| u.get_type() == UnitType::Zerg_Hatchery)
        .collect();
    // there's a different set of build checks for new bases, use a special building type
    let bt = UnitType::Special_Start_Location;
    let mut geysers = game.get_geysers();
    // sort geysers by how far they are from our hatcheries
    geysers.sort_by_cached_key(|g| hatches.iter().map(|h| g.get_distance(h)).min());
    // assume for now that each hatch is next to a unique geyser, so those
    // will be the closest and not where we should build the next hatch
    for g in geysers.iter().skip(hatches.len()) {
        let base_near_geyser = position_near(game, bt, builder, g.get_tile_position());
        if base_near_geyser.is_some() {
            return base_near_geyser;
        }
    }
    position_anywhere(game, bt, builder)
}

fn position_near(
    game: &Game,
    bt: UnitType,
    builder: &Unit,
    location: TilePosition,
) -> Option<BuildLoc> {
    let TilePosition { x, y } = location;
    let top = game.map_height();
    let side = game.map_width();

    // run four searches starting around the given location and going away from it
    building_pos_search(x, side, y, top, game, bt, builder) // quad I
        .or_else(|| building_pos_search(x, 0, y, top, game, bt, builder)) // quad II
        .or_else(|| building_pos_search(x, 0, y, 0, game, bt, builder)) // quad III
        .or_else(|| building_pos_search(x, side, y, 0, game, bt, builder)) // quad IV
}

fn position_anywhere(game: &Game, bt: UnitType, builder: &Unit) -> Option<BuildLoc> {
    building_pos_search(0, game.map_width(), 0, game.map_height(), game, bt, builder)
}

fn building_pos_search(
    x_start: i32,
    x_end: i32,
    y_start: i32,
    y_end: i32,
    game: &Game,
    bt: UnitType,
    builder: &Unit,
) -> Option<BuildLoc> {
    for y in y_start..y_end {
        for x in x_start..x_end {
            if game
                .can_build_here(builder, (x, y), bt, false)
                .unwrap_or(false)
            {
                return Some(BuildLoc { x, y });
            }
        }
    }
    None
}
