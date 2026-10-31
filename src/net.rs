//! Link to the browser's online-play code (web/online.js). On desktop there is
//! no online play, so every call here is a stub and the game stays local.

use crate::game::Player;

/// Message sent to ask for a rematch after a win. Cells use 0-8.
pub const REMATCH: u8 = 9;

#[cfg(target_arch = "wasm32")]
mod imp {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        fn vttt_status() -> i32;
        fn vttt_role() -> i32;
        fn vttt_recv() -> i32;
        fn vttt_send(v: i32);
        fn vttt_name_len(p: i32) -> i32;
        fn vttt_name_char(p: i32, i: i32) -> i32;
    }

    // Lets the JS loader check that its plugin version matches this crate.
    #[unsafe(no_mangle)]
    pub extern "C" fn vttt_net_crate_version() -> u32 {
        1
    }

    pub fn connected() -> bool {
        unsafe { vttt_status() == 2 }
    }
    pub fn role() -> i32 {
        unsafe { vttt_role() }
    }
    pub fn recv() -> i32 {
        unsafe { vttt_recv() }
    }
    pub fn send(v: i32) {
        unsafe { vttt_send(v) }
    }
    pub fn raw_name(p: i32) -> String {
        let len = unsafe { vttt_name_len(p) };
        (0..len.max(0))
            .filter_map(|i| {
                let c = unsafe { vttt_name_char(p, i) };
                if (0..=127).contains(&c) { Some(c as u8 as char) } else { None }
            })
            .collect()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    pub fn connected() -> bool {
        false
    }
    pub fn role() -> i32 {
        0
    }
    pub fn recv() -> i32 {
        -1
    }
    pub fn send(_v: i32) {}
    pub fn raw_name(_p: i32) -> String {
        String::new()
    }
}

/// True while an online game is connected.
pub fn connected() -> bool {
    imp::connected()
}

/// Which side this player controls: the host is X, the guest is O.
pub fn role() -> Option<Player> {
    match imp::role() {
        1 => Some(Player::X),
        2 => Some(Player::O),
        _ => None,
    }
}

/// Next message from the other player, if any.
pub fn recv() -> Option<u8> {
    u8::try_from(imp::recv()).ok()
}

pub fn send(v: u8) {
    imp::send(i32::from(v));
}

/// Raw player name typed on the home screen (empty on desktop).
pub fn raw_name(p: Player) -> String {
    imp::raw_name(match p {
        Player::X => 0,
        Player::O => 1,
    })
}
