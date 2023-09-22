use forward_dll::forward_dll;

fn main() {
    // 这个例子是测试 winmm.dll 仅导出 ordinal 的情况下是否正常。
    forward_dll("C:\\Windows\\system32\\winmm.dll").unwrap();
}
