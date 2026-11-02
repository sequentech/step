// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic tenants of the superadmin's tenant list.
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import {storyId, tenantRecord, type StoryRecord} from "@/__stories__/fixtures"

export const TENANT_RESOURCE = "sequent_backend_tenant"
export const ARCHIVED_TENANT_ID = storyId(1, 2)

export const tenantRecords = (): StoryRecord<Sequent_Backend_Tenant>[] => [
    {...tenantRecord, labels: {region: "north"}},
    {...tenantRecord, id: ARCHIVED_TENANT_ID, slug: "archived-council", is_active: false},
]
