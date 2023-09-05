use std::ffi::CString;

use windows_sys::Win32::{
    Foundation::{GetLastError, HMODULE},
    System::LibraryLoader::{
        FreeLibrary, GetModuleHandleExA, GetProcAddress, LoadLibraryA,
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    },
};

use crate::{ForwardError, ForwardResult};

/// 通过调用 GetModuleHandleExA 增加引用计数。
pub fn load_library_by_handle(inst: HMODULE) -> ForwardResult<HMODULE> {
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

/// LoadLibraryA 的包装。
pub fn load_library(lib_filename: &str) -> ForwardResult<HMODULE> {
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
pub fn free_library(inst: HMODULE) {
    unsafe { FreeLibrary(inst) };
}

/// 取得指定函数名称的函数地址。
pub fn get_proc_address_by_module(
    inst: HMODULE,
    proc_name: &str,
) -> ForwardResult<unsafe extern "system" fn() -> isize> {
    let proc_name = CString::new(proc_name).map_err(ForwardError::StringError)?;
    unsafe {
        GetProcAddress(inst, proc_name.as_ptr() as *const u8)
            .ok_or_else(|| ForwardError::Win32Error("GetProcAddress", GetLastError()))
    }
}

pub struct ForeignLibrary {
    pub module_handle: HMODULE,
}

impl ForeignLibrary {
    pub fn new(lib_name: &str) -> ForwardResult<Self> {
        let module_handle = load_library(lib_name)?;
        Ok(Self { module_handle })
    }

    pub fn get_proc_address(
        &self,
        proc_name: &str,
    ) -> ForwardResult<unsafe extern "system" fn() -> isize> {
        get_proc_address_by_module(self.module_handle, proc_name)
    }

    pub fn into_raw(self) -> HMODULE {
        let handle = self.module_handle;
        std::mem::forget(self);
        handle
    }
}

impl Drop for ForeignLibrary {
    fn drop(&mut self) {
        if self.module_handle != 0 {
            free_library(self.module_handle);
        }
    }
}
