#[no_mangle]
pub extern "system" fn DllMain(_inst: isize, reason: u32, _: *const u8) -> u32 {
    if reason == 1 {
        println!("==> version.dll loaded");
    }
    1
}
