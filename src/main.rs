use rsbwapi::*;

pub struct BotCallbacks;

impl AiModule for BotCallbacks {
    fn on_frame(&mut self, _game: &Game) {
        print!("hello BW frame");
    }
}

fn main() {
    rsbwapi::start(|_game| BotCallbacks);
}
