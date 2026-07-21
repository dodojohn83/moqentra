use std::path::{Path, PathBuf};

fn collect_protos(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_protos(&path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("proto") {
            out.push(path);
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);

    let manifest =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").ok_or("CARGO_MANIFEST_DIR not set")?);
    let workspace_root =
        manifest.parent().and_then(|p| p.parent()).ok_or("workspace root not found")?;
    let proto_root = workspace_root.join("proto");
    let proto_dir = proto_root.join("moqentra");

    let mut protos = Vec::new();
    collect_protos(&proto_dir, &mut protos)?;

    if protos.is_empty() {
        return Ok(());
    }

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("OUT_DIR not set")?);

    prost_build::Config::new()
        .out_dir(out_dir)
        .include_file("prost_generated.rs")
        .compile_protos(&protos, &[proto_root])
        .map_err(|e| e.into())
}
