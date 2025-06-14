use crate::prelude::*;

use windows_sys::Win32::System::Diagnostics::Debug::{
    FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_FROM_SYSTEM,
};

use crate::config;

type ScanCode = u16;

#[derive(Debug, Clone)]
pub enum KeyAction {
    Down(ScanCode),
    Up(ScanCode),
}

impl From<config::types::KeyAction> for KeyAction {
    fn from(action: config::types::KeyAction) -> Self {
        match action {
            config::types::KeyAction::Down(scan_code) => KeyAction::Down(scan_code),
            config::types::KeyAction::Up(scan_code) => KeyAction::Up(scan_code),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyStroke(Vec<KeyAction>);

impl From<Vec<config::types::KeyAction>> for KeyStroke {
    fn from(actions: Vec<config::types::KeyAction>) -> Self {
        KeyStroke(actions.into_iter().map(KeyAction::from).collect())
    }
}

#[derive(Debug, Clone)]
pub struct KeyStrokeButtonAction {
    key_stroke: KeyStroke,
}

impl KeyStrokeButtonAction {
    pub fn new(key_stroke: KeyStroke) -> Self {
        KeyStrokeButtonAction { key_stroke }
    }
}

impl MenuActionBehaviour<()> for KeyStrokeButtonAction {
    fn value(&self) {}

    fn on_change(&mut self, _value: ()) {
        if let Err(err) = send_keystroke(&self.key_stroke) {
            log::error!("Failed to send keystroke: {err}");
        }
    }
}

fn send_keystroke(key_stroke: &KeyStroke) -> Result<()> {
    let mut input: Vec<windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT> = Vec::new();

    for key_action in &key_stroke.0 {
        input.push(key_action_to_input(key_action));
    }

    send_input(&input)?;

    Ok(())
}

fn key_action_to_input(
    key_action: &KeyAction,
) -> windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT {
    let mut input = windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT {
        r#type: windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT_KEYBOARD,
        Anonymous: windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: windows_sys::Win32::UI::Input::KeyboardAndMouse::KEYBDINPUT {
                wVk: 0,
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    match key_action {
        KeyAction::Down(scan_code) => {
            input.Anonymous.ki.wScan = *scan_code;
            input.Anonymous.ki.dwFlags =
                windows_sys::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_SCANCODE;
        }
        KeyAction::Up(scan_code) => {
            input.Anonymous.ki.wScan = *scan_code;
            input.Anonymous.ki.dwFlags =
                windows_sys::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP
                    | windows_sys::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_SCANCODE;
        }
    }

    input
}

fn send_input(input: &[windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT]) -> Result<()> {
    let result = unsafe {
        windows_sys::Win32::UI::Input::KeyboardAndMouse::SendInput(
            u32::try_from(input.len())?,
            input.as_ptr(),
            i32::try_from(std::mem::size_of::<
                windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT,
            >())?,
        )
    };

    log::info!("SendInput result: {result}");

    if (result as usize) != input.len() {
        return Err(anyhow!("SendInput failed: {}", get_last_error()));
    }

    Ok(())
}

fn get_last_error() -> String {
    let error_code = unsafe { windows_sys::Win32::Foundation::GetLastError() };

    let error_message: *mut u16 = std::ptr::null_mut();

    let length = unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::FormatMessageW(
            FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM,
            std::ptr::null(),
            error_code,
            0,
            error_message,
            0,
            std::ptr::null(),
        )
    };

    if error_message.is_null() {
        // Failed to get error message
        format!("(Failed to retrieve error message for code: {error_code})")
    } else {
        let parts = unsafe { std::slice::from_raw_parts(error_message, length as usize) };

        let log_string = String::from_utf16(parts).unwrap_or(format!(
            "(Failed to get error message as string: {error_code})"
        ));

        log_string
    }
}
