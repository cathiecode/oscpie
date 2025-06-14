use crate::menu::MenuActionBehaviour;

#[derive(Debug, Clone)]
pub struct ExecOneShotButtonAction {
    program_path: String,
    args: Vec<String>,
}

impl ExecOneShotButtonAction {
    pub fn new(program_path: String, args: Vec<String>) -> Self {
        ExecOneShotButtonAction {
            program_path,
            args
        }
    }
}

impl MenuActionBehaviour<()> for ExecOneShotButtonAction {
    fn value(&self) {}

    fn on_change(&mut self, _value: ()) {
        std::process::Command::new(&self.program_path)
            .args(&self.args)
            .spawn()
            .map_err(|e| {
                log::error!("Failed to execute program {}: {}", self.program_path, e);
                e
            })
            .ok();
    }
}
