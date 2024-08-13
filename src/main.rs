use barcode::bot::BotCallbacks;

fn main() {
    rsbwapi::start(|_game| BotCallbacks::new());
}
