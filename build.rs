fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .out_dir("src")
        .compile(&["protos/main.proto"], &[""])?;
    Ok(())
}
