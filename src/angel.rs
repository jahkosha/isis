// SPDX-License-Identifier: EUPL-1.2

use anyhow::Result;

use crate::screensaver;

const TIMEOUT: u32 = 60000; // in millis

pub fn run() -> Result<()> {
    loop {
        let info = screensaver::query()?;
        let inhibiting = screensaver::inhibiting_idle().unwrap();
        if !inhibiting && info.state() == screensaver::XCB_SCREENSAVER_STATE_DISABLED {
            let ms_since_user_input = info.ms_since_user_input();
            if ms_since_user_input >= TIMEOUT {
                open();
            } else {
                let time_to_sleep = TIMEOUT - ms_since_user_input;
                std::thread::sleep(std::time::Duration::from_millis(time_to_sleep as u64));
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(TIMEOUT as u64 / 2));
        }
    }
    // FIXME How to exit? could we catch ctlr+c?
    // (handle termination properly --> uninhibit screeen if requiered)
    Ok(())
}

fn open() -> () {
    // FIXME How to resolve this path?
    std::process::Command::new("/mnt/data/homes/nixos/.cargo/bin/isis")
        .status()
        .expect("unable to run isis");
}
