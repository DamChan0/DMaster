mod apply;
mod display_info;
mod profile;
mod query;

use apply::apply_profile;
use profile::{load_profile, save_profile};
use query::get_display_info;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "apply" {
        let profile = load_profile("display_profile.json");
        apply_profile(&profile);
    } else {
        let profile = get_display_info();
        save_profile(&profile, "display_profile.json");
        println!("Saved profile to display_profile.json");
    }
    //TODO: add display serial number or UUID detection
    //TODO: add function to change display rendering mode (e.g, "mirror", "extend", etc.)
}
