// SPDX-License-Identifier: EUPL-1.2

use lockfree::channel::spsc;
use std::thread;

use isis::{angel, audio_analyzer, display};

pub fn main() -> () {
    match std::env::args().nth(1) {
        Some(arg) if arg == "--angel" => {
            angel::run().unwrap();
        }
        None => {
            let (mut event_tx, event_rx) = spsc::create();
            thread::spawn(move || audio_analyzer::run(&mut event_tx).unwrap());
            display::run(event_rx);
        }
        Some(arg) => {
            eprintln!("isis: {} is not an isis command.", arg);
        }
    }
}
