fn main() {
    let src_path = "src/api/proto/message.proto";
    prost_build::compile_protos(&[src_path], &["src/api/proto"]).unwrap();
}
