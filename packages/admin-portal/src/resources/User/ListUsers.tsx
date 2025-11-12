// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {List, Datagrid, TextField, EmailField, BooleanField} from "react-admin"

export interface ListUsersProps {
  electionEventId?: string
  electionId?: string
}

export const ListUsers: React.FC<ListUsersProps> = ({electionEventId, electionId}) => {
  const permanentFilters = {
    tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5", // Replace with actual tenant ID or get from context
    ...(electionEventId && {election_event_id: electionEventId}),
    ...(electionId && {election_id: electionId}),
  }

  return (
    <List
      resource="user"
      filter={permanentFilters}
      actions={false}
      pagination={false}
    >
      <Datagrid bulkActionButtons={false}>
        <TextField source="id" />
        <EmailField source="email" />
        <TextField source="first_name" />
        <TextField source="last_name" />
        <BooleanField source="enabled" />
        <BooleanField source="email_verified" />
      </Datagrid>
    </List>
  )
}