use crate::prelude::*;
use std::{
    collections::BTreeMap,
    fmt::Debug,
    sync::{Arc, Mutex, OnceLock},
};

static MESSAGES: OnceLock<Mutex<BTreeMap<&'static str, String>>> = OnceLock::new();

pub fn rt_debug<F>(id: &'static str, message: F)
where
    F: FnOnce() -> String,
{
    if MESSAGES.get().is_none() {
        return;
    }

    let mut messages = MESSAGES.wait().lock().unwrap();

    messages.insert(id, message());
}

pub fn debug_window() {
    loop {
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
        {
            let messages = MESSAGES
                .get_or_init(|| Mutex::new(BTreeMap::new()))
                .lock()
                .unwrap();

            messages.iter().for_each(|(id, message)| {
                println!("{}: {}", id, message);
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
