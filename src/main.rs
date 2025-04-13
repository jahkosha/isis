// SPDX-License-Identifier: EUPL-1.2

use lockfree::channel::spsc;
use std::thread;

use isis::audio_analyzer;
use isis::display;

pub fn main() -> () {
    let (mut event_tx, event_rx) = spsc::create();
    thread::spawn(move || audio_analyzer::run(&mut event_tx).unwrap());
    display::run(event_rx);
}
