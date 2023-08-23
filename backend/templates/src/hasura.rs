use graphql_client::{GraphQLQuery, Response};
use reqwest;

// The paths are relative to the directory where your `Cargo.toml` is located.
// Both json and the GraphQL schema language are supported as sources for the
// schema
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/query.graphql",
    response_derives = "Debug"
)]
pub struct GetTenant;

pub async fn perform_my_query(
    variables: get_tenant::Variables,
) -> Result<(), reqwest::Error> {
    let request_body = Query::build_query(variables);

    let client = reqwest::Client::new();
    let mut res = client
        .post("http://127.0.0.1:8080/v1/graphql")
        .header("X-My-Custom-Header", "foo")
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<query::ResponseData> = res.json().await?;
    println!("{:#?}", response_body);
    Ok(())
}
