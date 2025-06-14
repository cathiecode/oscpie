use std::process::Command;

fn main() {
    Command::new("deno")
        .arg("--allow-read")
        .arg("--allow-write")
        .arg("./utils/generate_action_codes.ts")
        .arg("./config/action_manifests.json")
        .arg("./src/openvr/input/prelude.rs")
        .arg("./src/openvr/input/generated.rs")
        .status()
        .expect("Failed to execute Deno script");
}
