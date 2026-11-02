// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// The applications of an event whose support reference (the one the pending
// page gives the voter) is the one searched. Compared on the server: the
// reference is derived from the id one way.
export const FIND_APPLICATIONS_BY_SUPPORT_REFERENCE = gql`
    query FindApplicationsBySupportReference(
        $electionEventId: uuid!
        $reference: String!
        $permissionLabel: String
    ) {
        find_applications_by_support_reference(
            election_event_id: $electionEventId
            reference: $reference
            permission_label: $permissionLabel
        ) {
            reference
            application_ids
        }
    }
`

export interface IFindApplicationsBySupportReferenceResult {
    find_applications_by_support_reference?: {
        reference: string
        application_ids: string[]
    } | null
}
