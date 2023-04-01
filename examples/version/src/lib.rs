use forward_dll::ForwardModule;

// ordinal 是一个可选的参数，用于生成编译器链接参数文件 `ordinal_link_args.txt`，注意，仅仅是生成链接参数文件，
// 如需在生成的 dll 中生效，还需要在 build.rs 中读取该文件，并将其内容打印出来作为链接器参数，参考 build.rs。
#[derive(ForwardModule)]
#[forward(target = "C:\\Windows\\system32\\version.dll", ordinal)]
pub struct VersionModule;

const VERSION_LIB: VersionModule = VersionModule;

#[no_mangle]
pub extern "system" fn DllMain(_inst: isize, reason: u32, _: *const u8) -> u32 {
    if reason == 1 {
        println!("==> version.dll loaded");
        VERSION_LIB.init().unwrap();
        println!("==> version.dll initialized");
    }
    1
}
