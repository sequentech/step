// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use std::fmt;
use tonic::{metadata::MetadataValue, transport::Channel, Request, Response, Streaming};
use tracing::{debug, info, instrument};

use crate::schema::immu_service_client::ImmuServiceClient;
use crate::schema::{
    CommittedSqlTx, Database, DatabaseListRequestV2, DatabaseListResponseV2, DeleteDatabaseRequest,
    LoginRequest, NamedParam, NewTxRequest, OpenSessionRequest, SqlExecRequest, SqlQueryRequest,
    SqlQueryResult, TxMode, UnloadDatabaseRequest,
};

pub struct Client {
    client: ImmuServiceClient<Channel>,
    username: String,
    password: String,
    auth_token: Option<String>,
    session_id: Option<String>,
}

// Client is embedded in other Debug-derived structures and tracing spans.
// Credentials and session identifiers must never become diagnostic output.
impl fmt::Debug for Client {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Client")
            .field("authenticated", &self.auth_token.is_some())
            .field("session_open", &self.session_id.is_some())
            .finish_non_exhaustive()
    }
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
            client,
            username: username.to_string(),
            password: password.to_string(),
            auth_token: None,
            session_id: None,
        })
    }

    #[instrument(skip_all, level = "debug")]
    pub async fn login(&mut self) -> Result<()> {
        let login_request = Request::new(LoginRequest {
            user: self.username.clone().into(),
            password: self.password.clone().into(),
        });
        // The pinned immudb API and existing board/audit callers use login + use_database.
        #[expect(
            deprecated,
            reason = "Retain token authentication required by current board/audit callers; migration to sessions is a separate protocol change"
        )]
        let response = self.client.login(login_request).await?;
        debug!("immudb login completed");
        self.auth_token = Some(format!("Bearer {}", response.get_ref().token));
        Ok(())
    }

    #[instrument(skip_all, level = "debug")]
    pub async fn logout(&mut self) -> Result<()> {
        let request = self.get_request(())?;
        #[expect(
            deprecated,
            reason = "Invalidate the token issued by the retained login API"
        )]
        self.client.logout(request).await?;
        debug!("immudb logout completed");
        self.auth_token = None;
        Ok(())
    }

    /// Creates an Authenticated request, with the proper Auth token
    fn get_request<T>(&self, data: T) -> Result<Request<T>> {
        let mut request = Request::new(data);
        if let Some(value) = &self.session_id {
            let mut session_id: MetadataValue<_> = value.parse()?;
            session_id.set_sensitive(true);
            request.metadata_mut().insert("sessionid", session_id);
        }
        if let Some(value) = &self.auth_token {
            let mut auth_token: MetadataValue<_> = value.parse()?;
            auth_token.set_sensitive(true);
            request.metadata_mut().insert("authorization", auth_token);
        }
        Ok(request)
    }

    #[instrument(skip_all)]
    pub async fn list_databases(&mut self) -> AsyncResponse<DatabaseListResponseV2> {
        let database_list_request = self.get_request(DatabaseListRequestV2 {})?;
        let database_list_response = self.client.database_list_v2(database_list_request).await?;
        debug!("immudb databases listed");
        Ok(database_list_response)
    }

    #[instrument(skip_all, level = "trace")]
    pub async fn has_database(&mut self, database_name: &str) -> Result<bool> {
        let database_list_request = self.get_request(DatabaseListRequestV2 {})?;
        let database_list_response = self.client.database_list_v2(database_list_request).await?;
        debug!("immudb databases listed");
        let has_database = database_list_response
            .get_ref()
            .databases
            .iter()
            .any(|database| database.name == database_name && database.loaded);
        Ok(has_database)
    }

    #[instrument(skip_all)]
    pub async fn has_tables(&mut self) -> Result<bool> {
        let list_tables_request = self.get_request(())?;
        let list_tables_response = self.client.list_tables(list_tables_request).await?;
        debug!("immudb tables listed");
        Ok(!list_tables_response.get_ref().rows.is_empty())
    }

    pub async fn sql_exec(&mut self, sql: &str, params: Vec<NamedParam>) -> Result<()> {
        let sql_exec_request = self.get_request(SqlExecRequest {
            sql: sql.into(),
            no_wait: false,
            params,
        })?;
        self.client.sql_exec(sql_exec_request).await?;
        debug!("immudb SQL execution completed");
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
        debug!("immudb transaction opened");
        Ok(new_tx_response.get_ref().transaction_id.clone())
    }

    /// Commits a transaction, returning the transaction results
    #[instrument(skip_all)]
    pub async fn commit(&mut self, transaction_id: &str) -> Result<CommittedSqlTx> {
        let mut commit_request = self.get_request(())?;
        let mut tx_id: MetadataValue<_> = transaction_id.parse()?;
        tx_id.set_sensitive(true);
        commit_request.metadata_mut().insert("transactionid", tx_id);
        let commit_response = self.client.commit(commit_request).await?;
        debug!("immudb transaction committed");
        Ok(commit_response.get_ref().clone())
    }

    /// Rolls back a transaction
    #[instrument(skip_all)]
    pub async fn rollback(&mut self, transaction_id: &str) -> Result<()> {
        let mut rollback_request = self.get_request(())?;
        let mut tx_id: MetadataValue<_> = transaction_id.parse()?;
        tx_id.set_sensitive(true);
        rollback_request
            .metadata_mut()
            .insert("transactionid", tx_id);
        self.client.rollback(rollback_request).await?;
        debug!("immudb transaction rolled back");
        Ok(())
    }

    pub async fn tx_sql_exec(
        &mut self,
        sql: &str,
        transaction_id: &str,
        params: Vec<NamedParam>,
    ) -> Result<()> {
        let mut sql_exec_request = self.get_request(SqlExecRequest {
            sql: sql.into(),
            no_wait: false,
            params,
        })?;
        let mut tx_id: MetadataValue<_> = transaction_id.parse()?;
        tx_id.set_sensitive(true);
        sql_exec_request
            .metadata_mut()
            .insert("transactionid", tx_id);

        self.client.tx_sql_exec(sql_exec_request).await?;
        debug!("immudb transaction SQL execution completed");
        Ok(())
    }

    pub async fn sql_query(
        &mut self,
        sql: &str,
        params: Vec<NamedParam>,
    ) -> AsyncResponse<SqlQueryResult> {
        let sql_query_request = self.get_request(SqlQueryRequest {
            sql: sql.into(),
            params,
            accept_stream: false,
            ..Default::default()
        })?;
        let sql_query_response = self.client.unary_sql_query(sql_query_request).await?;
        debug!("immudb SQL query completed");
        Ok(sql_query_response)
    }

    pub async fn streaming_sql_query(
        &mut self,
        sql: &str,
        params: Vec<NamedParam>,
    ) -> AsyncResponse<Streaming<SqlQueryResult>> {
        let sql_query_request = self.get_request(SqlQueryRequest {
            sql: sql.into(),
            params,
            accept_stream: true,
            ..Default::default()
        })?;
        let sql_query_response = self.client.sql_query(sql_query_request).await?;
        debug!("immudb SQL query completed");
        Ok(sql_query_response)
    }

    pub async fn tx_sql_query(
        &mut self,
        sql: &str,
        transaction_id: &str,
        params: Vec<NamedParam>,
    ) -> AsyncResponse<Streaming<SqlQueryResult>> {
        let mut sql_query_request = self.get_request(SqlQueryRequest {
            sql: sql.into(),
            params,
            accept_stream: false,
            ..Default::default()
        })?;
        let mut tx_id: MetadataValue<_> = transaction_id.parse()?;
        tx_id.set_sensitive(true);
        sql_query_request
            .metadata_mut()
            .insert("transactionid", tx_id);
        let sql_query_response = self.client.tx_sql_query(sql_query_request).await?;
        debug!("immudb transaction SQL query completed");
        Ok(sql_query_response)
    }

    #[instrument(skip_all)]
    pub async fn create_database(&mut self, database_name: &str) -> Result<()> {
        let create_db_request = self.get_request(crate::CreateDatabaseRequest {
            name: database_name.to_string(),
            settings: None,
            if_not_exists: true,
        })?;

        self.client.create_database_v2(create_db_request).await?;
        debug!("immudb database created");
        Ok(())
    }

    pub async fn use_database(&mut self, database_name: &str) -> Result<()> {
        let use_db_request = self.get_request(Database {
            database_name: database_name.to_string(),
        })?;

        let use_db_response = self.client.use_database(use_db_request).await?;
        debug!("immudb database selected");
        self.auth_token = Some(use_db_response.get_ref().token.clone());

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn delete_database(&mut self, database_name: &str) -> Result<()> {
        let unload_db_request = self
            .get_request(UnloadDatabaseRequest {
                database: database_name.to_string(),
            })
            .map_err(|err| anyhow!("Error generating the unload db request: {err:?}"))?;

        match self.client.unload_database(unload_db_request).await {
            Ok(_) => {
                info!("immudb database unloaded");
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
        self.client
            .delete_database(delete_db_request)
            .await
            .map_err(|err| anyhow!("Error deleting the database, status = {err:?}"))?;

        info!("immudb database deleted");
        Ok(())
    }

    pub async fn open_session(&mut self, database_name: &str) -> Result<()> {
        let open_session_request = Request::new(OpenSessionRequest {
            database_name: database_name.to_string(),
            username: self.username.clone().into(),
            password: self.password.clone().into(),
        });
        let open_session_response = self.client.open_session(open_session_request).await?;
        debug!("immudb session opened");
        self.session_id = Some(open_session_response.get_ref().session_id.clone());
        Ok(())
    }

    pub async fn close_session(&mut self) -> Result<()> {
        let close_session_request = self.get_request(())?;
        self.client.close_session(close_session_request).await?;
        debug!("immudb session closed");
        self.session_id = None;
        Ok(())
    }
}
