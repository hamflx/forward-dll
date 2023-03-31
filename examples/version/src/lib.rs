use forward_dll::ForwardModule;

#[derive(ForwardModule)]
#[forward("C:\\Windows\\system32\\version.dll")]
pub struct VersionModule;

#[no_mangle]
pub extern "system" fn DllMain(_inst: isize, reason: u32, _: *const u8) -> u32 {
    if reason == 1 {
        println!("==> version.dll loaded");
        let ver = VersionModule;
        ver.init().unwrap();
        println!("==> version.dll initialized");
    }
    1
}
