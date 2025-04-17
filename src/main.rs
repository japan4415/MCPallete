mod model;
mod config;
mod tui;

use config::*;
use model::*;
use tui::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ensure_config()?;
    let config = load_config().ok();
    if config.is_none() {
        eprintln!("[Error] Failed to load config file");
    }
    tui_main()?;
    Ok(())
}
