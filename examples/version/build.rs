use std::path::PathBuf;

// 本脚本仅在 #[forward(target = "...", ordinal)] 时，即指定了 ordinal 时才会有效，
// 将过程宏生成的 ordinal_link_args.txt 文件内容作为链接器参数。
fn main() {
    // 一般 OUT_DIR 为 target/debug/build/<crate-name>-<hash>/out，
    // 而 forward-dll-derive 包生成的 ordinal_link_args.txt 文件位于：target/debug，
    // 所以有 OUT_DIR 的 .parent().parent().parent()。
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap())
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .unwrap();
    let ordinal_file = out_dir.join("ordinal_link_args.txt");

    // 如果链接参数变化，则重新运行 build.rs。
    println!("cargo:rerun-if-changed={}", ordinal_file.display());

    // 读取 ordinal_link_args.txt，将内容作为链接器参数。
    if ordinal_file.exists() {
        let linker_flags = String::from_utf8(std::fs::read(ordinal_file).unwrap()).unwrap();
        for line in linker_flags.lines() {
            let line = line.trim();
            if !line.is_empty() {
                println!("cargo:rustc-link-arg={}", line);
            }
        }
    }
}
