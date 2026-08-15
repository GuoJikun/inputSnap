fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_manifest_file("app.manifest");
    res.set("ProductName", "InputSnap");
    res.set("FileDescription", "InputSnap - 输入法自动切换");
    res.set("CompanyName", "InputSnap");
    res.set("OriginalFilename", "input_snap.exe");
    // 版本号 0.1.0 -> 0x0001000000000000 (1.0.0.0)
    res.set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0001000000000000);
    res.set_version_info(winres::VersionInfo::FILEVERSION, 0x0001000000000000);
    res.compile().expect("Failed to compile Windows resource");
}
