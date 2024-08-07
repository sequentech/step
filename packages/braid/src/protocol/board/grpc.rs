


#[cfg(test)]
pub(crate) mod tests {

    use board_messages::grpc::server::B3Client;
    use board_messages::grpc::server::GetMessagesRequest;
    use serial_test::serial;

    #[tokio::test]
    #[ignore]
    #[serial]
    async fn test_grpc_client() {
        let mut client = B3Client::connect("http://[::1]:50051").await.unwrap();

        let request = tonic::Request::new(GetMessagesRequest {
            board: "default".to_string(),
            last_id: -1,
        });

        let response = client.get_messages(request).await.unwrap();

        println!("RESPONSE={:?}", response.into_inner().messages);
    }
}
