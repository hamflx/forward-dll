//! forward-dll 是一个辅助构造转发 DLL 的库。
//!
//! # Example 1
//!
//! 在 `build.rs` 中添加如下代码：
//!
//! ```rust
//! use forward_dll::forward_dll;
//!
//! forward_dll("C:\\Windows\\System32\\version.dll").unwrap();
//! ```
//!
//! 这将会读取目标 `DLL` 的导出表，然后使用 `cargo:rustc-*` 输出来链接到目标 DLL。这种方式可以连带 `ordinal` 一起转发。
//!
//! # Example 2
//!
//! 这种方式是在运行时动态加载目标 `DLL`，然后在导出的函数中，跳转到目标 `DLL` 的地址。
//!
//! ```rust
//! use forward_dll::ForwardModule;
//!
//! #[derive(ForwardModule)]
//! #[forward(target = "C:\\Windows\\system32\\version.dll")]
//! pub struct VersionModule;
//!
//! const VERSION_LIB: VersionModule = VersionModule;
//!
//! #[no_mangle]
//! pub extern "system" fn DllMain(_inst: isize, reason: u32, _: *const u8) -> u32 {
//!     if reason == 1 {
//!         println!("==> version.dll loaded");
//!         VERSION_LIB.init().unwrap();
//!         println!("==> version.dll initialized");
//!     }
//!     1
//! }
//! ```

pub mod utils;

use std::{collections::HashMap, ffi::NulError, path::PathBuf};

use implib::{def::ModuleDef, Flavor, ImportLibrary, MachineType};
use object::read::pe::{PeFile32, PeFile64};
use utils::ForeignLibrary;

pub use forward_dll_derive::ForwardModule;
use windows_sys::Win32::Foundation::HMODULE;

/// 由过程宏实现的 trait。
pub trait ForwardModule {
    /// 初始化转发相关的信息，如，加载目标 DLL 获取目标函数地址。
    fn init(&self) -> ForwardResult<()>;
}

#[doc(hidden)]
#[macro_export]
macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + $crate::count!($($xs)*));
}

/// 生成转发的导出函数，以及初始化方法，须在 DllMain 中调用初始化方法，以使生成的函数指向转发的目标函数。
///
/// # Examples
///
/// ```rust
/// forward_dll::forward_dll!(
///   "C:\\Windows\\system32\\version.dll",
///   DLL_VERSION_FORWARDER,
///   GetFileVersionInfoA
///   GetFileVersionInfoByHandle
///   GetFileVersionInfoExA
///   GetFileVersionInfoExW
///   GetFileVersionInfoSizeA
///   GetFileVersionInfoSizeExA
///   GetFileVersionInfoSizeExW
///   GetFileVersionInfoSizeW
///   GetFileVersionInfoW
///   VerFindFileA
///   VerFindFileW
///   VerInstallFileA
///   VerInstallFileW
///   VerLanguageNameA
///   VerLanguageNameW
///   VerQueryValueA
///   VerQueryValueW
/// );
///
/// #[no_mangle]
/// pub extern "system" fn DllMain(_inst: isize, reason: u32, _: *const u8) -> u32 {
///   if reason == 1 {
///     // 这里要自行持有底层的 version.dll 的句柄，防止被释放。
///     let _ = forward_dll::utils::load_library("C:\\Windows\\system32\\version.dll");
///     // 调用 forward_all 方法，建立导出函数与目标函数之间的映射关系。
///     let _ = unsafe { DLL_VERSION_FORWARDER.forward_all() };
///   }
///   1
/// }
/// ```
#[macro_export]
macro_rules! forward_dll {
    ($lib:expr, $name:ident, $($proc:ident)*) => {
        static mut $name: $crate::DllForwarder<{ $crate::count!($($proc)*) }> = $crate::DllForwarder {
            initialized: false,
            module_handle: 0,
            lib_name: $lib,
            target_functions_address: [
                0;
                $crate::count!($($proc)*)
            ],
            target_function_names: [
                $(stringify!($proc),)*
            ]
        };
        $crate::define_function!($lib, $name, 0, $($proc)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! define_function {
    ($lib:expr, $name:ident, $index:expr, ) => {};
    ($lib:expr, $name:ident, $index:expr, $export_name:ident = $proc:ident $($procs:tt)*) => {
        const _: () = {
            fn default_jumper(original_fn_addr: *const ()) -> usize {
                if original_fn_addr as usize != 0 {
                    return original_fn_addr as usize;
                }
                match $crate::utils::ForeignLibrary::new($lib) {
                    Ok(lib) => match lib.get_proc_address(std::stringify!($proc)) {
                        Ok(addr) => return addr as usize,
                        Err(err) => eprintln!("Error: {}", err)
                    }
                    Err(err) => eprintln!("Error: {}", err)
                }
                exit_fn as usize
            }

            fn exit_fn() {
                std::process::exit(1);
            }

            #[no_mangle]
            pub extern "system" fn $export_name() -> u32 {
                #[cfg(target_arch = "x86")]
                unsafe {
                    std::arch::asm!(
                        "push ecx",
                        "call eax",
                        "add esp, 4h",
                        "jmp eax",
                        in("eax") default_jumper,
                        in("ecx") $name.target_functions_address[$index],
                        options(nostack)
                    );
                }
                #[cfg(target_arch = "x86_64")]
                unsafe {
                    std::arch::asm!(
                        "push rcx",
                        "push rdx",
                        "push r8",
                        "push r9",
                        "push r10",
                        "push r11",
                        options(nostack)
                    );
                    std::arch::asm!(
                        "sub rsp, 28h",
                        "call rax",
                        "add rsp, 28h",
                        in("rax") default_jumper,
                        in("rcx") $name.target_functions_address[$index],
                        options(nostack)
                    );
                    std::arch::asm!(
                        "pop r11",
                        "pop r10",
                        "pop r9",
                        "pop r8",
                        "pop rdx",
                        "pop rcx",
                        "jmp rax",
                        options(nostack)
                    );
                }
                1
            }
        };
        $crate::define_function!($lib, $name, ($index + 1), $($procs)*);
    };
    ($lib:expr, $name:ident, $index:expr, $proc:ident $($procs:tt)*) => {
        $crate::define_function!($lib, $name, $index, $proc=$proc $($procs)*);
    };
}

#[derive(Debug)]
pub enum ForwardError {
    /// Win32 API 返回的错误。第一个值为调用的 Win32 API 函数名称，第二个为错误代码。
    Win32Error(&'static str, u32),
    /// 字符串编码错误。
    StringError(NulError),
    /// 已经初始化过了，不需要再次初始化。
    AlreadyInitialized,
}

impl std::fmt::Display for ForwardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ForwardError::Win32Error(func_name, err_code) => {
                write!(f, "Win32Error: {} {}", func_name, err_code)
            }
            ForwardError::StringError(ref err) => write!(f, "StringError: {}", err),
            ForwardError::AlreadyInitialized => write!(f, "AlreadyInitialized"),
        }
    }
}

impl std::error::Error for ForwardError {}

pub type ForwardResult<T> = std::result::Result<T, ForwardError>;

/// DLL 转发类型的具体实现。该类型不要自己实例化，应调用 forward_dll 宏生成具体的实例。
pub struct DllForwarder<const N: usize> {
    pub initialized: bool,
    pub module_handle: HMODULE,
    pub target_functions_address: [usize; N],
    pub target_function_names: [&'static str; N],
    pub lib_name: &'static str,
}

impl<const N: usize> DllForwarder<N> {
    /// 将所有函数的跳转地址设置为对应的 DLL 的同名函数地址。
    pub fn forward_all(&mut self) -> ForwardResult<()> {
        if self.initialized {
            return Err(ForwardError::AlreadyInitialized);
        }

        let lib = ForeignLibrary::new(self.lib_name)?;
        for index in 0..self.target_functions_address.len() {
            let addr_in_remote_module = lib.get_proc_address(self.target_function_names[index])?;
            self.target_functions_address[index] = addr_in_remote_module as *const usize as usize;
        }

        self.module_handle = lib.into_raw();
        self.initialized = true;

        Ok(())
    }
}

struct ExportItem {
    ordinal: u32,
    name: Option<String>,
}

/// 转发目标 `DLL` 的所有函数，同时会确保 `ordinal` 与目标函数一致。这个函数会读取目标 `DLL` 以获得导出函数信息，因此，要确保目标 `DLL` 在编译期存在。
pub fn forward_dll(dll_path: &str) -> Result<(), String> {
    forward_dll_with_dev_path(dll_path, dll_path)
}

/// 转发目标 `DLL` 的所有函数。与 `forward_dll` 类似，区别在于这个函数可以指定在编译时的目标 `DLL` 路径。
pub fn forward_dll_with_dev_path(dll_path: &str, dev_dll_path: &str) -> Result<(), String> {
    let exports = get_dll_export_names(dev_dll_path)?;
    forward_dll_impl(dll_path, exports.as_slice())
}

/// 转发目标 `DLL` 的所有函数。与 `forward_dll` 类似，区别在于这个函数不要求在编译期存在 dll。
pub fn forward_dll_with_exports(dll_path: &str, exports: &[(u32, &str)]) -> Result<(), String> {
    forward_dll_impl(
        dll_path,
        exports
            .iter()
            .map(|(ord, name)| ExportItem {
                ordinal: *ord,
                name: Some(name.to_string()),
            })
            .collect::<Vec<_>>()
            .as_slice(),
    )
}

fn forward_dll_impl(dll_path: &str, exports: &[ExportItem]) -> Result<(), String> {
    const SUFFIX: &str = ".dll";
    let dll_path_without_ext = if dll_path.to_ascii_lowercase().ends_with(SUFFIX) {
        &dll_path[..dll_path.len() - SUFFIX.len()]
    } else {
        dll_path
    };

    let out_dir = get_tmp_dir();

    // 有些导出符号没有名称，在编译的过程中，临时取一个符号名。
    let mut anonymous_map = HashMap::new();
    let mut anonymous_name_id = 0;

    // 输出链接参数，转发入口点到目标库。
    for ExportItem { name, ordinal } in exports {
        match name {
            Some(name) => println!(
                "cargo:rustc-link-arg=/EXPORT:{name}={dll_path_without_ext}.{name},@{ordinal}"
            ),
            None => {
                anonymous_name_id += 1;
                let fn_name = format!("forward_dll_anonymous_{anonymous_name_id}");
                println!(
                    "cargo:rustc-link-arg=/EXPORT:{fn_name}={dll_path_without_ext}.#{ordinal},@{ordinal}"
                );
                anonymous_map.insert(ordinal, fn_name);
            }
        };
    }

    // 构造 Import Library。
    let exports_def = String::from("LIBRARY version\nEXPORTS\n")
        + exports
            .iter()
            .map(|ExportItem { name, ordinal }| match name {
                Some(name) => format!("  {name} @{ordinal}\n"),
                None => {
                    let fn_name = anonymous_map.get(ordinal).unwrap();
                    format!("  {fn_name} @{ordinal} NONAME\n")
                }
            })
            .collect::<String>()
            .as_str();
    #[cfg(target_arch = "x86_64")]
    let machine = MachineType::AMD64;
    #[cfg(target_arch = "x86")]
    let machine = MachineType::I386;
    let mut def = ModuleDef::parse(&exports_def, machine)
        .map_err(|err| format!("ImportLibrary::new error: {err}"))?;
    for item in def.exports.iter_mut() {
        item.symbol_name = item.name.trim_start_matches('_').to_string();
    }
    let lib = ImportLibrary::from_def(def, machine, Flavor::Msvc);
    let version_lib_path = out_dir.join("version_proxy.lib");
    let mut lib_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(version_lib_path)
        .map_err(|err| format!("OpenOptions::open error: {err}"))?;
    lib.write_to(&mut lib_file)
        .map_err(|err| format!("ImportLibrary::write_to error: {err}"))?;

    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rustc-link-lib=version_proxy");

    Ok(())
}

/// 查询 OUT_DIR 变量，作为创建的 Import Library 存储路径。如果是在 doctest 的上下文中，是取不到 OUT_DIR 的。
fn get_tmp_dir() -> PathBuf {
    std::env::var("OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let dir = std::env::temp_dir().join("forward-dll-libs");
            if !dir.exists() {
                std::fs::create_dir_all(&dir).expect("Failed to create temp dir");
            }
            dir
        })
}

fn get_dll_export_names(dll_path: &str) -> Result<Vec<ExportItem>, String> {
    let dll_file = std::fs::read(dll_path).map_err(|err| format!("Failed to read file: {err}"))?;
    let in_data = dll_file.as_slice();

    let kind = object::FileKind::parse(in_data).map_err(|err| format!("Invalid file: {err}"))?;
    let exports = match kind {
        object::FileKind::Pe32 => PeFile32::parse(in_data)
            .map_err(|err| format!("Invalid pe file: {err}"))?
            .export_table()
            .map_err(|err| format!("Invalid pe file: {err}"))?
            .ok_or_else(|| "No export table".to_string())?
            .exports(),
        object::FileKind::Pe64 => PeFile64::parse(in_data)
            .map_err(|err| format!("Invalid pe file: {err}"))?
            .export_table()
            .map_err(|err| format!("Invalid pe file: {err}"))?
            .ok_or_else(|| "No export table".to_string())?
            .exports(),
        _ => return Err("Invalid file".to_string()),
    }
    .map_err(|err| format!("Invalid file: {err}"))?;

    let mut export_list = Vec::new();
    for export_item in exports {
        let ordinal = export_item.ordinal;
        let name = export_item
            .name
            .map(String::from_utf8_lossy)
            .map(String::from);
        let item = ExportItem { name, ordinal };
        export_list.push(item);
    }
    Ok(export_list)
}
