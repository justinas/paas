use prost_build::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.bytes(&["LogsResponse.lines"]);
    tonic_build::configure().compile_with_config(config, &["./proto/paas.proto"], &["./proto"])?;
    Ok(())
}
