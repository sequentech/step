use board_messages::grpc::pgsql::PgsqlConnectionParams;
use tonic::{transport::Server, Request, Response, Status};

use board_messages::grpc::server::PgsqlB3;
use board_messages::grpc::server::B3Server;

const PG_DATABASE: &'static str = "protocoldb";
const PG_HOST: &'static str = "localhost";
const PG_USER: &'static str = "postgres";
const PG_PASSW: &'static str = "postgrespw";
const PG_PORT: u32 = 49154;
const TEST_BOARD: &'static str = "testboard";


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
    
    let addr = "[::1]:50051".parse()?;
    let b3_impl = PgsqlB3::new(c, "protocoldb");

    Server::builder()
        .add_service(B3Server::new(b3_impl))
        .serve(addr)
        .await?;

    Ok(())
}