/// 将版本字符串（如 "1.0.0" 或 "1.0.0.0"）转换为 VERSIONINFO 资源所需的 64 位值。
/// 每段各占 16 位：major(16) | minor(16) | build(16) | revision(16)。
/// 未提供的段按 0 处理。
fn parse_version(version: &str) -> u64 {
    let parts: Vec<u64> = version
        .split('.')
        .map(|s| s.trim().parse::<u64>().unwrap_or(0))
        .collect();

    let get = |i: usize| parts.get(i).copied().unwrap_or(0);

    (get(0) << 48) | (get(1) << 32) | (get(2) << 16) | get(3)
}

fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_manifest_file("app.manifest");
    res.set_icon("assets/icon.ico");
    res.set("ProductName", "InputSnap");
    res.set("FileDescription", "InputSnap - 输入法自动切换");
    res.set("CompanyName", "InputSnap");
    res.set("OriginalFilename", "input_snap.exe");
    // 版本号统一从这里维护，如 "1.0.0"
    let version = parse_version("1.0.0");
    res.set_version_info(winres::VersionInfo::PRODUCTVERSION, version);
    res.set_version_info(winres::VersionInfo::FILEVERSION, version);
    res.compile().expect("Failed to compile Windows resource");
}
