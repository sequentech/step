// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_APPLICANT_ATTRIBUTES = gql`
    query GetApplicantAttributes($tenantId: uuid!, $applicationId: uuid!) {
        sequent_backend_applicant_attributes(
            where: {tenant_id: {_eq: $tenantId}, application_id: {_eq: $applicationId}}
        ) {
            applicant_attribute_name
            applicant_attribute_value
        }
    }
`
