use std::sync::{Arc, Mutex};

use crate::menu::MenuActionBehaviour;

#[derive(Debug, Clone)]
pub struct ExecOneShotButtonAction {
    program_path: String,
    args: Vec<String>,
    active: Arc<Mutex<bool>>,
}

impl ExecOneShotButtonAction {
    pub fn new(program_path: String, args: Vec<String>) -> Self {
        ExecOneShotButtonAction {
            program_path,
            args,
            active: Arc::new(Mutex::new(false)),
        }
    }
}

impl MenuActionBehaviour<bool> for ExecOneShotButtonAction {
    fn value(&self) -> bool {
        *self.active.lock().unwrap()
    }

    fn on_change(&mut self, _value: bool) {
        std::process::Command::new(&self.program_path)
            .args(&self.args)
            .spawn()
            .map_err(|e| {
                log::error!("Failed to execute program {}: {}", self.program_path, e);
                e
            })
            .ok();

        let active = self.active.clone();

        std::thread::spawn(move || {
            *active.lock().unwrap() = true;

            // Simulate some work
            std::thread::sleep(std::time::Duration::from_secs(3));

            *active.lock().unwrap() = false;
        });
    }
}
