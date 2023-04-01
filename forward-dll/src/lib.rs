pub mod utils;

use std::ffi::NulError;

use utils::ForeignLibrary;
use windows_sys::Win32::Foundation::HINSTANCE;

pub use forward_dll_derive::ForwardModule;

pub trait ForwardModule {
    fn init(&self) -> ForwardResult<()>;
}

#[macro_export]
macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + $crate::count!($($xs)*));
}

/// 生成转发的导出函数，以及初始化方法，须在 DllMain 中调用初始化方法，以使生成的函数指向转发的目标函数。
///
/// # Examples
///
/// ```
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
///     let _ = forward_dll::load_library("C:\\Windows\\system32\\version.dll");
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

#[macro_export]
macro_rules! define_function {
    ($lib:expr, $name:ident, $index:expr, ) => {};
    ($lib:expr, $name:ident, $index:expr, $proc:ident $($procs:ident)*) => {
        const _: () = {
            // 需要提前将指针计算出来，不然放在函数内的话，在 dev 模式编译会导致编译器生成 sub rsp, 20h 指令，而且会把字符串长度放在栈上。
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
            pub extern "system" fn $proc() -> u32 {
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
}

#[derive(Debug)]
pub enum ForwardError {
    Win32Error(&'static str, u32),
    StringError(NulError),
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
    pub module_handle: HINSTANCE,
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
