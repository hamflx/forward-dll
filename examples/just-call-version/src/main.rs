use widestring::U16CString;
use windows::{core::PCSTR, Win32::Storage::FileSystem::GetFileVersionInfoA};
use windows::core::PCWSTR;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Media::Audio::{PlaySoundW, SND_FILENAME};

fn main() {
    // 测试 version.dll 是否被加载。
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

    // 测试 winmm.dll 是否被加载。
    let filename = U16CString::from_str(concat!(env!("CARGO_MANIFEST_DIR"), "\\test.wav")).unwrap();
    unsafe {
        PlaySoundW(
            PCWSTR::from_raw(filename.as_ptr()),
            HMODULE::default(),
            SND_FILENAME
        );
    }
}
