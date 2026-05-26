fn main() {
    let mut config = prost_build::Config::new();
    config
        .compile_protos(&["proto/localharness.proto"], &["proto/"])
        .unwrap_or_else(|e| panic!("Failed to compile protos: {}", e));
}
