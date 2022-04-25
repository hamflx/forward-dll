# forward-dll

生成 DLL 转发的函数。

## 使用方法

```rust
forward_dll::forward_dll!(
  "version.dll",
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
    let _ = unsafe { DLL_VERSION_FORWARDER.forward_all() };
  }
  1
}
```

## License

[MIT](https://opensource.org/licenses/MIT)

Copyright (c) 2022-present, hamflx
