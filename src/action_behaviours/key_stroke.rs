use crate::prelude::*;

type ScanCode = u16;

#[derive(Debug, Clone)]
enum KeyAction {
    Down(ScanCode),
    Up(ScanCode),
}

type KeyStroke = Vec<KeyAction>;

#[derive(Debug, Clone)]
struct KeyStrokeButtonAction {
    key_stroke: KeyStroke,
    last_value: bool,
}

impl MenuActionBehaviour<bool> for KeyStrokeButtonAction {
    fn value(&self) -> bool {
        self.last_value
    }

    fn on_change(&mut self, value: bool) {
        if value != self.last_value && !value {
            send_keystroke(&self.key_stroke);
        }

        self.last_value = value;
    }
}

fn send_keystroke(key_stroke: &KeyStroke) {
    let mut input: Vec<windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT> = Vec::new();

    for key_action in key_stroke {
        input.push(key_action_to_input(key_action));
    }

    send_input(input);
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

fn send_input(input: Vec<windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT>) {
    unsafe {
        windows_sys::Win32::UI::Input::KeyboardAndMouse::SendInput(
            input.len() as u32,
            input.as_ptr(),
            std::mem::size_of::<windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32,
        );
    }
}
