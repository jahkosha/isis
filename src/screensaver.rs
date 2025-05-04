// SPDX-License-Identifier: EUPL-1.2

use xcb;

use anyhow::Result;
use dbus::blocking::Connection;
use dbus::Path;
use std::time::Duration;

pub const XCB_SCREENSAVER_STATE_OFF: u8 = 0;
pub const XCB_SCREENSAVER_STATE_ON: u8 = 1;
pub const XCB_SCREENSAVER_STATE_CYCLE: u8 = 2;
pub const XCB_SCREENSAVER_STATE_DISABLED: u8 = 3;

const INHIBIT_IDLE: u32 = 1 << 3; // 8

pub fn query() -> Result<xcb::screensaver::QueryInfoReply> {
    let (conn, screen_num) = xcb::Connection::connect(None)?;
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();
    let cookie = conn.send_request(&xcb::screensaver::QueryInfo {
        drawable: xcb::x::Drawable::Window(screen.root()),
    });
    Ok(conn.wait_for_reply(cookie)?)
}

pub fn inhibit(
    conn: &Connection,
    app_id: String,
    reason: String,
) -> Result<u32, Box<dyn std::error::Error>> {
    let proxy = conn.with_proxy(
        "org.gnome.SessionManager",
        "/org/gnome/SessionManager",
        Duration::from_millis(1000),
    );

    let toplevel_xid: u32 = 0;
    let flags: u32 = 8; // Inhibit idle/screensaver

    let (cookie,): (u32,) = proxy.method_call(
        "org.gnome.SessionManager",
        "Inhibit",
        (app_id, toplevel_xid, reason, flags),
    )?;

    Ok(cookie)
}

pub fn uninhibit(conn: &Connection, cookie: u32) -> Result<(), Box<dyn std::error::Error>> {
    let proxy = conn.with_proxy(
        "org.gnome.SessionManager",
        "/org/gnome/SessionManager",
        Duration::from_millis(1000),
    );

    let _: () = proxy.method_call("org.gnome.SessionManager", "Uninhibit", (cookie,))?;

    Ok(())
}

pub fn inhibiting_idle() -> Result<bool, Box<dyn std::error::Error>> {
    for obj in inhibitors()? {
        let flags = inhibitor_flags(&obj)?;
        let inhibit_idle: bool = flags & INHIBIT_IDLE != 0;
        if inhibit_idle {
            return Ok(true);
        }
    }
    return Ok(false);
}

fn inhibitor_flags(obj: &Path<'static>) -> Result<u32, Box<dyn std::error::Error>> {
    use dbus::blocking::BlockingSender;
    use dbus::blocking::Connection;
    use dbus::Message;

    let conn = Connection::new_session()?;
    let msg = Message::new_method_call(
        "org.gnome.SessionManager",
        obj,
        "org.gnome.SessionManager.Inhibitor",
        "GetFlags",
    )?;

    let reply = conn.send_with_reply_and_block(msg, Duration::from_millis(1000))?;
    let flags: u32 = reply.read1()?;
    Ok(flags)
}

fn inhibitors() -> Result<Vec<Path<'static>>, Box<dyn std::error::Error>> {
    use dbus::blocking::Connection;
    let conn = Connection::new_session()?;
    let proxy = conn.with_proxy(
        "org.gnome.SessionManager",
        "/org/gnome/SessionManager",
        Duration::from_millis(1000),
    );

    let (inhibitors,): (Vec<Path<'static>>,) =
        proxy.method_call("org.gnome.SessionManager", "GetInhibitors", ())?;

    Ok(inhibitors)
}
