use std::ffi::{CString, NulError};

use windows_sys::Win32::{
    Foundation::{GetLastError, HINSTANCE},
    System::LibraryLoader::{
        FreeLibrary, GetModuleHandleExA, GetProcAddress, LoadLibraryA,
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    },
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
        forward_dll::define_function!($lib, $name, 0, $($proc)*);
    };
}

#[macro_export]
macro_rules! define_function {
    ($lib:expr, $name:ident, $index:expr, ) => {};
    ($lib:expr, $name:ident, $index:expr, $proc:ident $($procs:ident)*) => {
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
                    in("rax") forward_dll::default_jumper,
                    in("rcx") std::concat!($lib, "\0").as_ptr() as usize,
                    in("rdx") std::concat!(std::stringify!($proc), "\0").as_ptr() as usize,
                    in("r8") $name.target_functions_address[$index],
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
        forward_dll::define_function!($lib, $name, ($index + 1), $($procs)*);
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
        let module_handle = load_library(self.lib_name)?;

        for index in 0..self.target_functions_address.len() {
            let addr_in_remote_module =
                get_proc_address_by_module(module_handle, self.target_function_names[index])?;
            self.target_functions_address[index] = addr_in_remote_module as *const usize as usize;
        }

        free_library(module_handle);

        Ok(())
    }
}

/// 通过调用 GetModuleHandleExA 增加引用计数。
pub fn load_library_by_handle(inst: HINSTANCE) -> ForwardResult<HINSTANCE> {
    let mut module_handle = 0;
    let pin_success = unsafe {
        GetModuleHandleExA(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            inst as *const u8,
            &mut module_handle,
        )
    } != 0;
    if !pin_success {
        return Err(ForwardError::Win32Error("GetModuleHandleExA", unsafe {
            GetLastError()
        }));
    }
    Ok(module_handle)
}

/// 默认的跳板，如果没有执行初始化操作，则进入该函数。
pub fn default_jumper(
    lib_name: *const u8,
    func_name: *const u8,
    original_fn_addr: *const (),
) -> usize {
    if original_fn_addr as usize != 0 {
        return original_fn_addr as usize;
    }

    let module_handle = unsafe { LoadLibraryA(lib_name) };
    if module_handle != 0 {
        let addr = unsafe { GetProcAddress(module_handle, func_name) };
        unsafe { FreeLibrary(module_handle) };
        return addr.map(|addr| addr as usize).unwrap_or(exit_fn as usize);
    }

    exit_fn as usize
}

pub fn exit_fn() {
    std::process::exit(1);
}

/// LoadLibraryA 的包装。
pub fn load_library(lib_filename: &str) -> ForwardResult<HINSTANCE> {
    let module_name = CString::new(lib_filename).map_err(ForwardError::StringError)?;
    let module_handle = unsafe { LoadLibraryA(module_name.as_ptr() as *const u8) };
    if module_handle == 0 {
        return Err(ForwardError::Win32Error("LoadLibraryA", unsafe {
            GetLastError()
        }));
    }
    Ok(module_handle)
}

/// FreeLibrary 的包装。
pub fn free_library(inst: HINSTANCE) {
    unsafe { FreeLibrary(inst) };
}

fn get_proc_address_by_module(
    inst: HINSTANCE,
    proc_name: &str,
) -> ForwardResult<unsafe extern "system" fn() -> isize> {
    let proc_name = CString::new(proc_name).map_err(ForwardError::StringError)?;
    unsafe {
        GetProcAddress(inst, proc_name.as_ptr() as *const u8)
            .ok_or_else(|| ForwardError::Win32Error("GetProcAddress", GetLastError()))
    }
}
