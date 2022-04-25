use std::ffi::{CString, NulError};

use windows_sys::Win32::{
    Foundation::{GetLastError, HINSTANCE},
    System::LibraryLoader::{GetProcAddress, LoadLibraryA},
};

#[macro_export]
macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + forward_dll::count!($($xs)*));
}

/// 生成转发的导出函数，以及初始化方法，须在 DllMain 中调用初始化方法，以使生成的函数指向转发的目标函数。
///
/// # Examples
///
/// ```
/// forward_dll::forward_dll!(
///   "version.dll",
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
///     let _ = unsafe { DLL_VERSION_FORWARDER.forward_all() };
///   }
///   1
/// }
/// ```
#[macro_export]
macro_rules! forward_dll {
    ($lib:expr, $name:ident, $($proc:ident)*) => {
        static mut $name: forward_dll::DllForwarder<{ forward_dll::count!($($proc)*) }> = forward_dll::DllForwarder {
            lib_name: $lib,
            target_functions_address: [
                0;
                forward_dll::count!($($proc)*)
            ],
            target_function_names: [
                $(stringify!($proc),)*
            ]
        };
        forward_dll::define_function!($name, 0, $($proc)*);
    };
}

#[macro_export]
macro_rules! define_function {
    ($name:ident, $index:expr, ) => {};
    ($name:ident, $index:expr, $proc:ident $($procs:ident)*) => {
        #[no_mangle]
        pub extern "system" fn $proc() -> u32 {
            unsafe {
                std::arch::asm!(
                    "jmp rax",
                    in("rax") $name.target_functions_address[$index],
                    options(nostack)
                );
            }
            1
        }
        forward_dll::define_function!($name, ($index + 1), $($procs)*);
    };
}

#[derive(Debug)]
pub enum ForwardError {
    Win32Error(&'static str, u32),
    StringError(NulError),
}

impl std::fmt::Display for ForwardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ForwardError::Win32Error(func_name, err_code) => {
                write!(f, "Win32Error: {} {}", func_name, err_code)
            }
            ForwardError::StringError(ref err) => write!(f, "StringError: {}", err),
        }
    }
}

impl std::error::Error for ForwardError {}

pub type ForwardResult<T> = std::result::Result<T, ForwardError>;

/// DLL 转发类型的具体实现。该类型不要自己实例化，应调用 forward_dll 宏生成具体的实例。
pub struct DllForwarder<const N: usize> {
    pub target_functions_address: [usize; N],
    pub target_function_names: [&'static str; N],
    pub lib_name: &'static str,
}

impl<const N: usize> DllForwarder<N> {
    /// 将所有函数的跳转地址设置为对应的 DLL 的同名函数地址。
    pub fn forward_all(&mut self) -> ForwardResult<()> {
        let load_module_dir = "C:\\Windows\\System32\\";
        let module_full_path = format!("{}{}", load_module_dir, self.lib_name);
        let module_handle = get_module_handle(module_full_path.as_str())?;

        for index in 0..self.target_functions_address.len() {
            let addr_in_remote_module =
                get_proc_address_by_module(module_handle, self.target_function_names[index])?;
            self.target_functions_address[index] = addr_in_remote_module as *const usize as usize;
        }

        Ok(())
    }
}

pub fn get_module_handle(module_full_path: &str) -> ForwardResult<HINSTANCE> {
    let module_name = CString::new(module_full_path).map_err(ForwardError::StringError)?;
    let module_handle = unsafe { LoadLibraryA(module_name.as_ptr() as *const u8) };
    if module_handle == 0 {
        return Err(ForwardError::Win32Error("LoadLibraryA", unsafe {
            GetLastError()
        }));
    }
    Ok(module_handle)
}

pub fn get_proc_address_by_module(
    inst: HINSTANCE,
    proc_name: &str,
) -> ForwardResult<unsafe extern "system" fn() -> isize> {
    let proc_name = CString::new(proc_name).map_err(ForwardError::StringError)?;
    unsafe {
        GetProcAddress(inst, proc_name.as_ptr() as *const u8)
            .ok_or_else(|| ForwardError::Win32Error("GetProcAddress", GetLastError()))
    }
}
