use forward_dll::forward_dll;

fn main() {
    forward_dll("C:\\Windows\\System32\\version.dll", true).unwrap();
}
