// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! Offer both native file URLs and text, so egui's text-only Paste event is emitted.
use std::{io, sync::mpsc, thread, time::Duration};
use x11rb::{
    COPY_DEPTH_FROM_PARENT, CURRENT_TIME, NONE,
    connection::{Connection, RequestConnection},
    protocol::{Event, xproto::*},
    wrapper::ConnectionExt as _,
};
fn err(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}
pub fn publish(text: String) -> io::Result<()> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("mucrab-file-selection-owner".into())
        .spawn(move || {
            let result = (|| -> io::Result<()> {
                let (c, screen) = x11rb::connect(None).map_err(err)?;
                let window = c.generate_id().map_err(err)?;
                c.create_window(
                    COPY_DEPTH_FROM_PARENT,
                    window,
                    c.setup().roots[screen].root,
                    0,
                    0,
                    1,
                    1,
                    0,
                    WindowClass::INPUT_OUTPUT,
                    0,
                    &CreateWindowAux::new(),
                )
                .map_err(err)?
                .check()
                .map_err(err)?;
                let atom = |name: &str| -> io::Result<u32> {
                    Ok(c.intern_atom(false, name.as_bytes())
                        .map_err(err)?
                        .reply()
                        .map_err(err)?
                        .atom)
                };
                let clipboard = atom("CLIPBOARD")?;
                let targets = atom("TARGETS")?;
                let uri = atom("text/uri-list")?;
                let utf8 = atom("UTF8_STRING")?;
                let plain = atom("text/plain")?;
                let plain_utf8 = atom("text/plain;charset=utf-8")?;
                let gnome = atom("x-special/gnome-copied-files")?;
                if text.len() + 128 > c.maximum_request_bytes() {
                    return Err(err(
                        "File clipboard exceeds this X11 server's single-transfer limit",
                    ));
                }
                let offered = [targets, uri, utf8, plain, plain_utf8, gnome];
                c.set_selection_owner(window, clipboard, CURRENT_TIME)
                    .map_err(err)?
                    .check()
                    .map_err(err)?;
                if c.get_selection_owner(clipboard)
                    .map_err(err)?
                    .reply()
                    .map_err(err)?
                    .owner
                    != window
                {
                    return Err(err("Could not claim file clipboard"));
                }
                c.flush().map_err(err)?;
                let _ = tx.send(Ok(()));
                loop {
                    match c.wait_for_event().map_err(err)? {
                        Event::SelectionClear(_) => break,
                        Event::SelectionRequest(q) => {
                            let property = if q.property == NONE {
                                q.target
                            } else {
                                q.property
                            };
                            let accepted = if q.target == targets {
                                c.change_property32(
                                    PropMode::REPLACE,
                                    q.requestor,
                                    property,
                                    AtomEnum::ATOM,
                                    &offered,
                                )
                                .map_err(err)?
                                .check()
                                .is_ok()
                            } else if offered.contains(&q.target) {
                                let data = if q.target == gnome {
                                    format!("copy\n{}", text.replace("\r\n", "\n"))
                                } else {
                                    text.clone()
                                };
                                c.change_property8(
                                    PropMode::REPLACE,
                                    q.requestor,
                                    property,
                                    q.target,
                                    data.as_bytes(),
                                )
                                .map_err(err)?
                                .check()
                                .is_ok()
                            } else {
                                false
                            };
                            let event = SelectionNotifyEvent {
                                response_type: SELECTION_NOTIFY_EVENT,
                                sequence: 0,
                                time: q.time,
                                requestor: q.requestor,
                                selection: q.selection,
                                target: q.target,
                                property: if accepted { property } else { NONE },
                            };
                            // A disappearing requestor must not terminate clipboard ownership.
                            if let Ok(cookie) =
                                c.send_event(false, q.requestor, EventMask::NO_EVENT, event)
                            {
                                let _ = cookie.check();
                            }
                            c.flush().map_err(err)?;
                        }
                        _ => {}
                    }
                }
                Ok(())
            })();
            if let Err(e) = result {
                let _ = tx.send(Err(e));
            }
        })
        .map_err(err)?;
    rx.recv_timeout(Duration::from_secs(5)).map_err(err)?
}
