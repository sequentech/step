// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use std::fmt::Debug;
use tonic::{metadata::MetadataValue, transport::Channel, Request, Response, Streaming};
use tracing::{debug, info, instrument};

use crate::schema::immu_service_client::ImmuServiceClient;
use crate::schema::{
    CommittedSqlTx, Database, DatabaseListRequestV2, DatabaseListResponseV2, DeleteDatabaseRequest,
    LoginRequest, NamedParam, NewTxRequest, OpenSessionRequest, SqlExecRequest, SqlQueryRequest,
    SqlQueryResult, TxMode, UnloadDatabaseRequest,
};

#[derive(Debug)]
pub struct Client {
    client: ImmuServiceClient<Channel>,
    username: String,
    password: String,
    auth_token: Option<String>,
    session_id: Option<String>,
}

pub type AsyncResponse<T> = Result<Response<T>>;

/// Represents a Immudb Client.
/// Allows you to handle operations in an easier manner.
impl Client {
    #[instrument(skip(password), level = "trace")]
    pub async fn new(server_url: &str, username: &str, password: &str) -> Result<Client> {
        let mut client = ImmuServiceClient::connect(String::from(server_url)).await?;
        client = client.max_encoding_message_size(134217728);
        client = client.max_decoding_message_size(134217728);

        Ok(Client {
            client: client,
            username: username.to_string(),
            password: password.to_string(),
            auth_token: None,
            session_id: None,
        })
    }

    #[instrument(level = "debug")]
    pub async fn login(&mut self) -> Result<()> {
        let login_request = Request::new(LoginRequest {
            user: self.username.clone().into(),
            password: self.password.clone().into(),
        });
        let response = self.client.login(login_request).await?;
        debug!("grpc-login-response={:?}", response);
        self.auth_token = Some(format!("Bearer {}", response.get_ref().token));
        Ok(())
    }

    #[instrument(level = "debug")]
    pub async fn logout(&mut self) -> Result<()> {
        let request = self.get_request(())?;
        let response = self.client.logout(request).await?;
        debug!("grpc-login-response={:?}", response);
        self.auth_token = None;
        Ok(())
    }

    /// Creates an Authenticated request, with the proper Auth token
    fn get_request<T: Debug>(&self, data: T) -> Result<Request<T>> {
        let mut request = Request::new(data);

        if self.session_id.is_some() {
            let session_id: MetadataValue<_> =
                self.session_id.clone().expect("impossible").parse()?;
            request.metadata_mut().insert("sessionid", session_id);
        }

        if self.auth_token.is_some() {
            let auth_token: MetadataValue<_> =
                self.auth_token.clone().expect("impossible").parse()?;
            request.metadata_mut().insert("authorization", auth_token);
        }

        Ok(request)
    }

    #[instrument]
    pub async fn list_databases(&mut self) -> AsyncResponse<DatabaseListResponseV2> {
        let database_list_request = self.get_request(DatabaseListRequestV2 {})?;
        let database_list_response = self.client.database_list_v2(database_list_request).await?;
        debug!("grpc-database-list-response={:?}", database_list_response);
        Ok(database_list_response)
    }

    #[instrument(level = "trace")]
    pub async fn has_database(&mut self, database_name: &str) -> Result<bool> {
        let database_list_request = self.get_request(DatabaseListRequestV2 {})?;
        let database_list_response = self.client.database_list_v2(database_list_request).await?;
        debug!("grpc-database-list-response={:?}", database_list_response);
        let has_database = database_list_response
            .get_ref()
            .databases
            .iter()
            .find(|database| database.name == database_name)
            .is_some();
        Ok(has_database)
    }

    #[instrument]
    pub async fn has_tables(&mut self) -> Result<bool> {
        let list_tables_request = self.get_request(())?;
        let list_tables_response = self.client.list_tables(list_tables_request).await?;
        debug!("list-tables-response={:?}", list_tables_response);
        Ok(!list_tables_response.get_ref().rows.is_empty())
    }

    pub async fn sql_exec(&mut self, sql: &str, params: Vec<NamedParam>) -> Result<()> {
        let sql_exec_request = self.get_request(SqlExecRequest {
            sql: sql.into(),
            no_wait: false,
            params: params,
        })?;
        let sql_exec_response = self.client.sql_exec(sql_exec_request).await?;
        debug!("sql-exec-response={:?}", sql_exec_response);
        Ok(())
    }

    /// Creates a new transaction, returning the transaction id
    pub async fn new_tx(&mut self, mode: TxMode) -> Result<String> {
        let new_tx_request = self.get_request(NewTxRequest {
            mode: mode.into(),
            snapshot_must_include_tx_id: None,
            snapshot_renewal_period: None,
            unsafe_mvcc: false,
        })?;
        let new_tx_response = self.client.new_tx(new_tx_request).await?;
        debug!("new-tx-response={:?}", new_tx_response);
        Ok(new_tx_response.get_ref().transaction_id.clone())
    }

    /// Commits a transaction, returning the transaction results
    #[instrument(skip(self))]
    pub async fn commit(&mut self, transaction_id: &String) -> Result<CommittedSqlTx> {
        let mut commit_request = self.get_request(())?;
        let tx_id: MetadataValue<_> = transaction_id.parse()?;
        commit_request.metadata_mut().insert("transactionid", tx_id);
        let commit_response = self.client.commit(commit_request).await?;
        debug!("commit-response={:?}", commit_response);
        Ok(commit_response.get_ref().clone())
    }

    /// Rolls back a transaction
    #[instrument(skip(self))]
    pub async fn rollback(&mut self, transaction_id: &String) -> Result<()> {
        let mut rollback_request = self.get_request(())?;
        let tx_id: MetadataValue<_> = transaction_id.parse()?;
        rollback_request
            .metadata_mut()
            .insert("transactionid", tx_id);
        let rollback_response = self.client.rollback(rollback_request).await?;
        debug!("rollback-response={:?}", rollback_response);
        Ok(())
    }

    pub async fn tx_sql_exec(
        &mut self,
        sql: &str,
        transaction_id: &String,
        params: Vec<NamedParam>,
    ) -> Result<()> {
        let mut sql_exec_request = self.get_request(SqlExecRequest {
            sql: sql.into(),
            no_wait: false,
            params: params,
        })?;
        let tx_id: MetadataValue<_> = transaction_id.parse()?;
        sql_exec_request
            .metadata_mut()
            .insert("transactionid", tx_id);

        let sql_exec_response = self.client.tx_sql_exec(sql_exec_request).await?;
        debug!("tx-sql-exec-response={:?}", sql_exec_response);
        Ok(())
    }

    pub async fn sql_query(
        &mut self,
        sql: &str,
        params: Vec<NamedParam>,
    ) -> AsyncResponse<SqlQueryResult> {
        let sql_query_request = self.get_request(SqlQueryRequest {
            sql: sql.into(),
            params: params,
            reuse_snapshot: false,
            accept_stream: false,
        })?;
        let sql_query_response = self.client.unary_sql_query(sql_query_request).await?;
        debug!("sql-query-response={:?}", sql_query_response);
        Ok(sql_query_response)
    }

    pub async fn streaming_sql_query(
        &mut self,
        sql: &str,
        params: Vec<NamedParam>,
    ) -> AsyncResponse<Streaming<SqlQueryResult>> {
        let sql_query_request = self.get_request(SqlQueryRequest {
            sql: sql.into(),
            params: params,
            reuse_snapshot: false,
            accept_stream: true,
        })?;
        let sql_query_response = self.client.sql_query(sql_query_request).await?;
        debug!("sql-query-response={:?}", sql_query_response);
        Ok(sql_query_response)
    }

    pub async fn tx_sql_query(
        &mut self,
        sql: &str,
        transaction_id: &String,
        params: Vec<NamedParam>,
    ) -> AsyncResponse<Streaming<SqlQueryResult>> {
        let mut sql_query_request = self.get_request(SqlQueryRequest {
            sql: sql.into(),
            params: params,
            reuse_snapshot: false,
            accept_stream: false,
        })?;
        let tx_id: MetadataValue<_> = transaction_id.parse()?;
        sql_query_request
            .metadata_mut()
            .insert("transactionid", tx_id);
        let sql_query_response = self.client.tx_sql_query(sql_query_request).await?;
        debug!("tx-sql-query-response={:?}", sql_query_response);
        Ok(sql_query_response)
    }

    #[instrument]
    pub async fn create_database(&mut self, database_name: &str) -> Result<()> {
        let create_db_request = self.get_request(crate::CreateDatabaseRequest {
            name: database_name.to_string(),
            settings: None,
            if_not_exists: true,
        })?;

        let create_db_response = self.client.create_database_v2(create_db_request).await?;
        debug!("grpc-create-database-response={:?}", create_db_response);
        Ok(())
    }

    pub async fn use_database(&mut self, database_name: &str) -> Result<()> {
        let use_db_request = self.get_request(Database {
            database_name: database_name.to_string(),
        })?;

        let use_db_response = self.client.use_database(use_db_request).await?;
        debug!("grpc-use-database-response={:?}", use_db_response);
        self.auth_token = Some(use_db_response.get_ref().token.clone());

        Ok(())
    }

    #[instrument]
    pub async fn delete_database(&mut self, database_name: &str) -> Result<()> {
        let unload_db_request = self
            .get_request(UnloadDatabaseRequest {
                database: database_name.to_string(),
            })
            .map_err(|err| anyhow!("Error generating the unload db request: {err:?}"))?;

        match self.client.unload_database(unload_db_request).await {
            Ok(unload_db_response) => {
                info!("grpc-unload-database-response={unload_db_response:?}");
            }
            Err(err) => {
                if err.message() == "database does not exist" {
                    info!("database is already removed");
                    return Ok(());
                } else {
                    return Err(anyhow!("Error unloading the database, status = {err:?}"));
                }
            }
        };

        let delete_db_request = self
            .get_request(DeleteDatabaseRequest {
                database: database_name.to_string(),
            })
            .map_err(|err| anyhow!("Error generating the delete db request: {err:?}"))?;
        let delete_db_response = self
            .client
            .delete_database(delete_db_request)
            .await
            .map_err(|err| anyhow!("Error unloading the database, status = {err:?}"));

        info!("grpc-delete-database-response={delete_db_response:?}");
        Ok(())
    }

    pub async fn open_session(&mut self, database_name: &str) -> Result<()> {
        let open_session_request = Request::new(OpenSessionRequest {
            database_name: database_name.to_string(),
            username: self.username.clone().into(),
            password: self.password.clone().into(),
        });
        let open_session_response = self.client.open_session(open_session_request).await?;
        debug!("grpc-open-session-response={open_session_response:?}");
        self.session_id = Some(open_session_response.get_ref().session_id.clone());
        Ok(())
    }

    pub async fn close_session(&mut self) -> Result<()> {
        let close_session_request = self.get_request(())?;
        let close_session_response = self.client.close_session(close_session_request).await?;
        debug!("grpc-open-session-response={close_session_response:?}");
        self.session_id = None;
        Ok(())
    }
}
