fn main() {
    tonic_build::configure()
        .build_server(true)
        .compile(
            &["../rpc.proto"],
            &[".."],
        )
        .unwrap();
}