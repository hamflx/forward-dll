# forward-dll

生成 DLL 转发的函数。

## 使用方法——静态转发

先在 `Cargo.toml` 中添加依赖：

```toml
[build-dependencies]
forward-dll = "0.1.11"
```

在 `build.rs` 中添加如下代码：

```rust
use forward_dll::forward_dll;

fn main() {
  forward_dll("C:\\Windows\\System32\\version.dll").unwrap();

  // 32 位目标。
  // forward_dll("C:\\Windows\\SysWOW64\\version.dll").unwrap();
}
```

## 自定义转发

```rust
use forward_dll::forward_dll_with_exports;
forward_dll_with_exports(
    "C:\\Windows\\system32\\version.dll",
    &[
        (1, "GetFileVersionInfoA"),
        (2, "GetFileVersionInfoByHandle"),
        (3, "GetFileVersionInfoExA"),
        (4, "GetFileVersionInfoExW"),
        (5, "GetFileVersionInfoSizeA"),
        (6, "GetFileVersionInfoSizeExA"),
        (7, "GetFileVersionInfoSizeExW"),
        (8, "GetFileVersionInfoSizeW"),
        (9, "GetFileVersionInfoW"),
        (10, "VerFindFileA"),
        (11, "VerFindFileW"),
        (12, "VerInstallFileA"),
        (13, "VerInstallFileW"),
        (14, "VerLanguageNameA"),
        (15, "VerLanguageNameW"),
        (16, "VerQueryValueA"),
        (17, "VerQueryValueW"),
    ],
)
.unwrap();
```

## 使用方法——动态转发

```rust
use forward_dll::ForwardModule;

#[derive(ForwardModule)]
#[forward(target = "C:\\Windows\\system32\\version.dll")]
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
```

**注意，`#[forward(target = "path/of/your/dll")]` 中的路径，应在编译期可以访问到（过程宏会读取这个文件并提取出导出表），如果这个路径为相对路径，则应相对于 `Cargo.toml` 所在的目录。**

## v0.1.5 及之前版本的使用方法

```rust
forward_dll::forward_dll!(
  "C:\\Windows\\system32\\version.dll",
  DLL_VERSION_FORWARDER,
  GetFileVersionInfoA
  GetFileVersionInfoByHandle
  GetFileVersionInfoExA
  GetFileVersionInfoExW
  GetFileVersionInfoSizeA
  GetFileVersionInfoSizeExA
  GetFileVersionInfoSizeExW
  GetFileVersionInfoSizeW
  GetFileVersionInfoW
  VerFindFileA
  VerFindFileW
  VerInstallFileA
  VerInstallFileW
  VerLanguageNameA
  VerLanguageNameW
  VerQueryValueA
  VerQueryValueW
);

#[no_mangle]
pub extern "system" fn DllMain(_inst: isize, reason: u32, _: *const u8) -> u32 {
  if reason == 1 {
    // 这里要自行持有底层的 version.dll 的句柄，防止被释放。
    let _ = forward_dll::utils::load_library("C:\\Windows\\system32\\version.dll");
    // 调用 forward_all 方法，建立导出函数与目标函数之间的映射关系。
    let _ = unsafe { DLL_VERSION_FORWARDER.forward_all() };
  }
  1
}
```

## 运行 example

```powershell
cargo build -p version; cargo run -p just-call-version
```

## License

[MIT](https://opensource.org/licenses/MIT)

Copyright (c) 2022-present, hamflx
