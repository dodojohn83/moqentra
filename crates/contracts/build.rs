use std::path::{Path, PathBuf};

fn collect_protos(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read proto dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_protos(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("proto") {
            out.push(path);
        }
    }
}

fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc binary available");
    std::env::set_var("PROTOC", protoc);

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().unwrap().parent().unwrap();
    let proto_root = workspace_root.join("proto");
    let proto_dir = proto_root.join("moqentra");

    let mut protos = Vec::new();
    collect_protos(&proto_dir, &mut protos);

    if protos.is_empty() {
        return;
    }

    let proto_strings: Vec<&str> = protos.iter().map(|p| p.to_str().unwrap()).collect();
    let includes: Vec<&str> = vec![proto_root.to_str().unwrap()];

    prost_build::Config::new()
        .out_dir(PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")))
        .include_file("prost_generated.rs")
        .compile_protos(&proto_strings, &includes)
        .expect("prost compile protos");
}
