use board_messages::grpc::B3Client;
use board_messages::grpc::GetMessagesRequest;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = B3Client::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(GetMessagesRequest {
        board: "default".to_string(),
        last_id: -1,
    });

    let response = client.get_messages(request).await?;

    println!("RESPONSE={:?}", response.into_inner().messages);

    Ok(())
}