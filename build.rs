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

/// 将版本号补齐为 4 段（manifest 的 assemblyIdentity 要求 major.minor.build.revision 格式）
fn four_part_version(version: &str) -> String {
    let mut parts: Vec<&str> = version.split('.').take(4).collect();
    while parts.len() < 4 {
        parts.push("0");
    }
    parts.join(".")
}

fn main() {
    // manifest 模板内容变化时需要重新执行构建脚本
    println!("cargo:rerun-if-changed=app.manifest");

    // 用 CARGO_MANIFEST_DIR 拼接绝对路径，避免 winres 生成的 rc 文件
    // 以 OUT_DIR 为工作目录时找不到相对路径下的资源
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();

    // 版本号自动取自 Cargo.toml（CARGO_PKG_VERSION 由 Cargo 在构建时注入），只需维护一处
    let pkg_version = env!("CARGO_PKG_VERSION");

    // 读取 manifest 模板，将版本占位符替换为 4 段版本号后写入 OUT_DIR
    let template_path = std::path::Path::new(&manifest_dir).join("app.manifest");
    let template = std::fs::read_to_string(&template_path).expect("读取 app.manifest 失败");
    let manifest_content = template.replace("%VERSION%", &four_part_version(pkg_version));
    let manifest_path = std::path::Path::new(&out_dir).join("app.manifest");
    std::fs::write(&manifest_path, manifest_content).expect("写入生成的 app.manifest 失败");

    let icon_path = std::path::Path::new(&manifest_dir).join("assets/icon.ico");

    let mut res = winres::WindowsResource::new();
    res.set_manifest_file(manifest_path.to_str().unwrap());
    res.set_icon(icon_path.to_str().unwrap());
    res.set("ProductName", "InputSnap");
    res.set("FileDescription", "InputSnap - 输入法自动切换");
    res.set("CompanyName", "InputSnap");
    res.set("OriginalFilename", "input_snap.exe");
    let version = parse_version(pkg_version);
    res.set_version_info(winres::VersionInfo::PRODUCTVERSION, version);
    res.set_version_info(winres::VersionInfo::FILEVERSION, version);
    res.compile().expect("Failed to compile Windows resource");
}
