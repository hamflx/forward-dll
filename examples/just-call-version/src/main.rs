use windows::{core::PCSTR, Win32::Storage::FileSystem::GetFileVersionInfoA};

fn main() {
    let mut buf = vec![0u8; 0x1000];
    let ok = unsafe {
        GetFileVersionInfoA(
            PCSTR::from_raw(b"C:\\Windows\\System32\\version.dll\0".as_ptr()),
            0,
            buf.len() as _,
            buf.as_mut_ptr() as _,
        )
    }
    .as_bool();
    println!("==> just-call-version: GetFileVersionInfoA={:?}", ok);
}
