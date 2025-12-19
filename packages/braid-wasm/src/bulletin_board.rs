// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use b4::api_types::{
    ConfirmMessageRequest, ConfirmMessageResponse, GetMessageResponse, InitiateMessageRequest,
    InitiateMessageResponse, ListMessagesResponse,
};
use wasm_bindgen::prelude::*;

pub struct BulletinBoardClient {
    base_url: String,
    client: reqwest::Client,
}

impl BulletinBoardClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn initiate_message(
        &self,
        size: usize,
        sender_pk: String,
        statement_kind: String,
        batch: i32,
        mix_number: i32,
    ) -> Result<InitiateMessageResponse, JsValue> {
        let url = format!("{}/messages/initiate", self.base_url);
        let request = InitiateMessageRequest {
            size,
            sender_pk,
            statement_kind,
            batch,
            mix_number,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| JsValue::from_str(&format!("HTTP request failed: {}", e)))?;

        response
            .json::<InitiateMessageResponse>()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to parse response: {}", e)))
    }

    pub async fn confirm_message(
        &self,
        message_id: &str,
        data: Option<Vec<u8>>,
        sender_pk: String,
        statement_kind: String,
        batch: i32,
        mix_number: i32,
    ) -> Result<ConfirmMessageResponse, JsValue> {
        let url = format!("{}/messages/{}/confirm", self.base_url, message_id);
        let request = ConfirmMessageRequest {
            data,
            sender_pk,
            statement_kind,
            batch,
            mix_number,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| JsValue::from_str(&format!("HTTP request failed: {}", e)))?;

        response
            .json::<ConfirmMessageResponse>()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to parse response: {}", e)))
    }

    pub async fn get_message(&self, message_id: &str) -> Result<GetMessageResponse, JsValue> {
        let url = format!("{}/messages/{}", self.base_url, message_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| JsValue::from_str(&format!("HTTP request failed: {}", e)))?;

        response
            .json::<GetMessageResponse>()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to parse response: {}", e)))
    }

    pub async fn list_messages(&self) -> Result<ListMessagesResponse, JsValue> {
        let url = format!("{}/messages", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| JsValue::from_str(&format!("HTTP request failed: {}", e)))?;

        response
            .json::<ListMessagesResponse>()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to parse response: {}", e)))
    }
}
