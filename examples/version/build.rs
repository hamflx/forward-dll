use forward_dll::forward_dll;

fn main() {
    forward_dll("C:\\Windows\\system32\\version.dll").unwrap();

    // 32 位目标。
    // forward_dll("C:\\Windows\\SysWOW64\\version.dll").unwrap();
}
