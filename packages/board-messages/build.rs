fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("src/grpc/proto/b3.proto")?;
    Ok(())
}