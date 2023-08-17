use anyhow::{anyhow, Result};
use tracing::{debug, instrument};

use std::fmt::Debug;

use immudb_rs::immu_service_client::ImmuServiceClient;
use immudb_rs::{
    Database,
    DatabaseListRequestV2,
    DatabaseListResponseV2,
    DeleteDatabaseRequest,
    LoginRequest,
    SqlExecRequest,
    SqlQueryRequest,
    SqlQueryResult,
    UnloadDatabaseRequest,
};
use tonic::{
    metadata::MetadataValue,
    transport::Channel,
    Request,
    Response,
};

#[derive(Debug)]
pub struct Board {
    client: ImmuServiceClient<Channel>,
    auth_token: Option<String>,
}

type AsyncResponse<T> = Result<Response<T>>;

impl Board {
    #[instrument]
    pub async fn new(
        server_url: &str,
    ) -> Result<Board> {
        let client = ImmuServiceClient::connect(String::from(server_url)).await?;
        Ok(Board {
            client: client,
            auth_token: None,
        })
    }

    #[instrument]
    async fn login(
        &mut self, username: &str, password: &str
    ) -> Result<()> {
        let login_request = Request::new(LoginRequest {
            user: username.clone().into(),
            password: password.clone().into()
        });
        let response = self.client.login(login_request).await?;
        debug!("grpc-login-response={:?}", response);
        self.auth_token = Some(format!("Bearer {}", response.get_ref().token));
        Ok(())
    }

    /// Creates an Authenticated request, with the proper Auth token
    #[instrument]
    fn get_request<T: Debug>(
        &self,
        data: T
    ) -> Result<Request<T>>
    {
        let mut request = Request::new(data);
        let token: MetadataValue<_> = self.auth_token
            .clone()
            .ok_or(anyhow!("not logged in",))?
            .parse()?;
        request.metadata_mut().insert("authorization", token);
        return Ok(request);
    }

    #[instrument]
    pub async fn list_boards(&mut self)
        -> AsyncResponse<DatabaseListResponseV2>
    {
        let database_list_request = self.get_request(
            DatabaseListRequestV2 {}
        )?;
        let database_list_response = self.client
            .database_list_v2(database_list_request)
            .await?;
        Ok(database_list_response)
    }

    #[instrument]
    pub async fn board_exists(&mut self, board_name: &str) -> Result<bool> {
        let database_list_request = self.get_request(
            DatabaseListRequestV2 {}
        )?;
        let database_list_response = self.client
            .database_list_v2(database_list_request)
            .await?;
        Ok(database_list_response
            .get_ref()
            .databases
            .iter()
            .find(|database| database.name == board_name).is_some()
        )
    }

    #[instrument]
    pub async fn has_tables(&mut self) -> Result<bool> {
        let list_tables_request = self.get_request(())?;
        let list_tables_response = self.client
            .list_tables(list_tables_request)
            .await?;
        debug!("list_tables_response={:?}", list_tables_response);
        Ok(list_tables_response.get_ref().rows.is_empty())
    }

    #[instrument]
    pub async fn sql_exec(&mut self, sql: &str) -> Result<()> {
        let sql_exec_request = self.get_request(
            SqlExecRequest {
                sql: sql.clone().into(),
                no_wait: true,
                params: vec![],
            }
        )?;
        let sql_exec_response = self.client
            .sql_exec(sql_exec_request)
            .await?;
        debug!("sql_exec_response={:?}", sql_exec_response);
        Ok(())
    }

    #[instrument]
    pub async fn sql_query(
        &mut self, sql: &str
    ) -> AsyncResponse<SqlQueryResult>
    {
        let sql_query_request = self.get_request(
            SqlQueryRequest {
                sql: sql.clone().into(),
                reuse_snapshot: false,
                params: vec![],
            }
        )?;
        let sql_query_response = self.client
            .sql_query(sql_query_request)
            .await?;
        debug!("sql_query_response={:?}", sql_query_response);
        Ok(sql_query_response)
    }

    #[instrument]
    pub async fn use_board(&mut self, board_name: &str) -> Result<()> {
        let use_db_request = self.get_request(
            Database { database_name: board_name.to_string() }
        )?;

        let use_db_response = self.client.use_database(use_db_request).await?;
        debug!("grpc-use-response={:?}", use_db_response);
        self.auth_token =  Some(use_db_response.get_ref().token.clone());

        Ok(())
    }

    #[instrument]
    pub async fn delete_board(&mut self, board_name: &str) -> Result<()>
    {
        let unload_db_request = self.get_request(
            UnloadDatabaseRequest { database: board_name.to_string() },
        )?;
        let _unload_db_response = self.client
            .unload_database(unload_db_request).await?;
        debug!("Deleting index database...");
        let delete_db_request = self.get_request(
            DeleteDatabaseRequest { database: board_name.to_string() },
        )?;
        let _delete_db_response = self.client
            .delete_database(delete_db_request).await?;
        debug!("Index Database deleted!");
        Ok(())
    }
}
