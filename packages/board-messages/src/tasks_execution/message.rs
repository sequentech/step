// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use strand::serialization::StrandSerialize;

use crate::tasks_execution::newtypes::*;
use crate::tasks_execution::statement::Statement;
use crate::tasks_execution::statement::StatementBody;
use crate::tasks_execution::statement::StatementHead;
// use crate::electoral_log::newtypes::EventIdString;

// use super::newtypes::*;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, std::fmt::Debug)]
pub struct Message {
    pub statement: Statement,
}

impl Message {
    pub fn post_task_message(
        tenant_id: &str,
        election_event_id: &str,
        task_type: &str,
    ) -> Result<Self> {
        let tenant = TenantIdString(tenant_id.to_string()); //TODO: understand if i need
        let election_event = ElectionEventIdString(election_event_id.to_string());
        let task_type = TaskExecutionType(task_type.to_string());

        let body = StatementBody::startTask(tenant, election_event.clone(), task_type);
        Self::from_body(election_event.clone(), body)
    }

    ////////////////////////////

    fn from_body(event: ElectionEventIdString, body: StatementBody) -> Result<Self> {
        let head = StatementHead::from_body(event, &body);
        let statement = Statement::new(head, body);

        Ok(Message {
            statement,
        })

    }
}
