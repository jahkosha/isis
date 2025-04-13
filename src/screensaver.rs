// SPDX-License-Identifier: EUPL-1.2

use xcb;

use anyhow::Result;

pub fn reset() -> Result<()> {
    let (conn, _screen_num) = xcb::Connection::connect(None)?;
    conn.send_request(&xcb::x::ForceScreenSaver {
        mode: xcb::x::ScreenSaver::Reset,
    });
    conn.flush()?;
    Ok(())
}
