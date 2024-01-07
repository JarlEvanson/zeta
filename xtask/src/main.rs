//! An integrated utility script meant to simplify testing and updates to the zeta project and its binaries.

use config::App;

use clap::Parser;

mod config;
mod config_checksum;

fn main() {
    let app = App::parse();
    app.execute();
}
