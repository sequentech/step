// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IPermissions} from "@/types/keycloak"
import {GraphQLRequest} from "@apollo/client"
import {isUndefined} from "@sequentech/ui-core"

const AdminOperationMap: Record<string, IPermissions> = {
    // area
    sequent_backend_area: IPermissions.AREA_READ,
    sequent_backend_areas: IPermissions.AREA_READ,
    insert_sequent_backend_area: IPermissions.AREA_WRITE,
    insert_sequent_backend_areas: IPermissions.AREA_WRITE,
    update_sequent_backend_area: IPermissions.AREA_WRITE,
    update_sequent_backend_areas: IPermissions.AREA_WRITE,
    delete_sequent_backend_area: IPermissions.AREA_WRITE,
    delete_sequent_backend_areas: IPermissions.AREA_WRITE,
    // area_contest
    sequent_backend_area_contest: IPermissions.AREA_READ,
    sequent_backend_area_contests: IPermissions.AREA_READ,
    insert_sequent_backend_area_contest: IPermissions.AREA_WRITE,
    insert_sequent_backend_area_contests: IPermissions.AREA_WRITE,
    update_sequent_backend_area_contest: IPermissions.AREA_WRITE,
    update_sequent_backend_area_contests: IPermissions.AREA_WRITE,
    delete_sequent_backend_area_contest: IPermissions.AREA_WRITE,
    delete_sequent_backend_area_contests: IPermissions.AREA_WRITE,
    // ballot_publication
    sequent_backend_ballot_publication: IPermissions.PUBLISH_READ,
    sequent_backend_ballot_publications: IPermissions.PUBLISH_READ,
    insert_sequent_backend_ballot_publication: IPermissions.PUBLISH_WRITE,
    insert_sequent_backend_ballot_publications: IPermissions.PUBLISH_WRITE,
    update_sequent_backend_ballot_publication: IPermissions.PUBLISH_WRITE,
    update_sequent_backend_ballot_publications: IPermissions.PUBLISH_WRITE,
    delete_sequent_backend_ballot_publication: IPermissions.PUBLISH_WRITE,
    delete_sequent_backend_ballot_publications: IPermissions.PUBLISH_WRITE,
    // ballot_style
    sequent_backend_ballot_style: IPermissions.PUBLISH_READ,
    sequent_backend_ballot_styles: IPermissions.PUBLISH_READ,
    insert_sequent_backend_ballot_style: IPermissions.PUBLISH_WRITE,
    insert_sequent_backend_ballot_styles: IPermissions.PUBLISH_WRITE,
    update_sequent_backend_ballot_style: IPermissions.PUBLISH_WRITE,
    update_sequent_backend_ballot_styles: IPermissions.PUBLISH_WRITE,
    delete_sequent_backend_ballot_style: IPermissions.PUBLISH_WRITE,
    delete_sequent_backend_ballot_styles: IPermissions.PUBLISH_WRITE,
    // candidate
    sequent_backend_candidate: IPermissions.CANDIDATE_READ,
    sequent_backend_candidates: IPermissions.CANDIDATE_READ,
    insert_sequent_backend_candidate: IPermissions.CANDIDATE_CREATE,
    insert_sequent_backend_candidates: IPermissions.CANDIDATE_CREATE,
    update_sequent_backend_candidate: IPermissions.CANDIDATE_WRITE,
    update_sequent_backend_candidates: IPermissions.CANDIDATE_WRITE,
    delete_sequent_backend_candidate: IPermissions.CANDIDATE_DELETE,
    delete_sequent_backend_candidates: IPermissions.CANDIDATE_DELETE,
    // cast_vote
    sequent_backend_cast_vote: IPermissions.CAST_VOTE_READ,
    sequent_backend_cast_votes: IPermissions.CAST_VOTE_READ,
    // template
    sequent_backend_template: IPermissions.template_READ,
    sequent_backend_templates: IPermissions.template_READ,
    insert_sequent_backend_template: IPermissions.template_WRITE,
    insert_sequent_backend_templates: IPermissions.template_WRITE,
    update_sequent_backend_template: IPermissions.template_WRITE,
    update_sequent_backend_templates: IPermissions.template_WRITE,
    delete_sequent_backend_template: IPermissions.template_WRITE,
    delete_sequent_backend_templates: IPermissions.template_WRITE,
    // contest
    sequent_backend_contest: IPermissions.CONTEST_READ,
    sequent_backend_contests: IPermissions.CONTEST_READ,
    insert_sequent_backend_contest: IPermissions.CONTEST_CREATE,
    insert_sequent_backend_contests: IPermissions.CONTEST_CREATE,
    update_sequent_backend_contest: IPermissions.CONTEST_WRITE,
    update_sequent_backend_contests: IPermissions.CONTEST_WRITE,
    delete_sequent_backend_contest: IPermissions.CONTEST_DELETE,
    delete_sequent_backend_contests: IPermissions.CONTEST_DELETE,
    // document
    sequent_backend_document: IPermissions.DOCUMENT_READ,
    sequent_backend_documents: IPermissions.DOCUMENT_READ,
    insert_sequent_backend_document: IPermissions.DOCUMENT_WRITE,
    insert_sequent_backend_documents: IPermissions.DOCUMENT_WRITE,
    update_sequent_backend_document: IPermissions.DOCUMENT_WRITE,
    update_sequent_backend_documents: IPermissions.DOCUMENT_WRITE,
    delete_sequent_backend_document: IPermissions.DOCUMENT_WRITE,
    delete_sequent_backend_documents: IPermissions.DOCUMENT_WRITE,
    // election
    sequent_backend_election: IPermissions.ELECTION_READ,
    sequent_backend_elections: IPermissions.ELECTION_READ,
    insert_sequent_backend_election: IPermissions.ELECTION_CREATE,
    insert_sequent_backend_elections: IPermissions.ELECTION_CREATE,
    update_sequent_backend_election: IPermissions.ELECTION_WRITE,
    update_sequent_backend_elections: IPermissions.ELECTION_WRITE,
    delete_sequent_backend_election: IPermissions.ELECTION_DELETE,
    delete_sequent_backend_elections: IPermissions.ELECTION_DELETE,
    // election_type
    sequent_backend_election_type: IPermissions.ELECTION_TYPE_READ,
    sequent_backend_elections_type: IPermissions.ELECTION_TYPE_READ,
    insert_sequent_backend_election_type: IPermissions.ELECTION_TYPE_WRITE,
    insert_sequent_backend_election_types: IPermissions.ELECTION_TYPE_WRITE,
    update_sequent_backend_election_type: IPermissions.ELECTION_TYPE_WRITE,
    update_sequent_backend_election_types: IPermissions.ELECTION_TYPE_WRITE,
    delete_sequent_backend_election_type: IPermissions.ELECTION_TYPE_WRITE,
    delete_sequent_backend_election_types: IPermissions.ELECTION_TYPE_WRITE,
    // election_event
    sequent_backend_election_event: IPermissions.ELECTION_EVENT_READ,
    sequent_backend_election_events: IPermissions.ELECTION_EVENT_READ,
    insert_sequent_backend_election_event: IPermissions.ELECTION_EVENT_CREATE,
    insert_sequent_backend_election_events: IPermissions.ELECTION_EVENT_CREATE,
    update_sequent_backend_election_event: IPermissions.ELECTION_EVENT_WRITE,
    update_sequent_backend_election_events: IPermissions.ELECTION_EVENT_WRITE,
    delete_sequent_backend_election_event: IPermissions.ELECTION_EVENT_DELETE,
    delete_sequent_backend_election_events: IPermissions.ELECTION_EVENT_DELETE,
    // keys_ceremony
    sequent_backend_keys_ceremony: IPermissions.ADMIN_CEREMONY,
    sequent_backend_keys_ceremonys: IPermissions.ADMIN_CEREMONY,
    // results_area_contest
    sequent_backend_results_area_contest: IPermissions.TALLY_RESULTS_READ,
    sequent_backend_results_area_contests: IPermissions.TALLY_RESULTS_READ,
    // results_area_contest_candidate
    sequent_backend_results_area_contest_candidate: IPermissions.TALLY_RESULTS_READ,
    sequent_backend_results_area_contest_candidates: IPermissions.TALLY_RESULTS_READ,
    // results_contest
    sequent_backend_results_contest: IPermissions.TALLY_RESULTS_READ,
    sequent_backend_results_contests: IPermissions.TALLY_RESULTS_READ,
    // results_contest_candidate
    sequent_backend_results_contest_candidate: IPermissions.TALLY_RESULTS_READ,
    sequent_backend_results_contest_candidates: IPermissions.TALLY_RESULTS_READ,
    // results_election
    sequent_backend_results_election: IPermissions.TALLY_RESULTS_READ,
    sequent_backend_results_elections: IPermissions.TALLY_RESULTS_READ,
    // results_election
    sequent_backend_results_election_area: IPermissions.TALLY_RESULTS_READ,
    sequent_backend_results_election_areas: IPermissions.TALLY_RESULTS_READ,
    // results_event
    sequent_backend_results_event: IPermissions.TALLY_RESULTS_READ,
    sequent_backend_results_events: IPermissions.TALLY_RESULTS_READ,
    // support_material
    sequent_backend_support_material: IPermissions.SUPPORT_MATERIAL_READ,
    sequent_backend_support_materials: IPermissions.SUPPORT_MATERIAL_READ,
    insert_sequent_backend_support_material: IPermissions.SUPPORT_MATERIAL_WRITE,
    insert_sequent_backend_support_materials: IPermissions.SUPPORT_MATERIAL_WRITE,
    update_sequent_backend_support_material: IPermissions.SUPPORT_MATERIAL_WRITE,
    update_sequent_backend_support_materials: IPermissions.SUPPORT_MATERIAL_WRITE,
    delete_sequent_backend_support_material: IPermissions.SUPPORT_MATERIAL_WRITE,
    delete_sequent_backend_support_materials: IPermissions.SUPPORT_MATERIAL_WRITE,
    // tally_session
    sequent_backend_tally_session: IPermissions.ADMIN_CEREMONY,
    sequent_backend_tally_sessions: IPermissions.ADMIN_CEREMONY,
    // tally_session_contest
    sequent_backend_tally_session_contest: IPermissions.ADMIN_CEREMONY,
    sequent_backend_tally_session_contests: IPermissions.ADMIN_CEREMONY,
    // tally_session_execution
    sequent_backend_tally_session_execution: IPermissions.ADMIN_CEREMONY,
    sequent_backend_tally_session_executions: IPermissions.ADMIN_CEREMONY,
    // tally_sheet
    sequent_backend_tally_sheet: IPermissions.TALLY_SHEET_VIEW,
    sequent_backend_tally_sheets: IPermissions.TALLY_SHEET_VIEW,
    insert_sequent_backend_tally_sheet: IPermissions.TALLY_SHEET_CREATE,
    insert_sequent_backend_tally_sheets: IPermissions.TALLY_SHEET_CREATE,
    update_sequent_backend_tally_sheet: IPermissions.TALLY_SHEET_CREATE,
    update_sequent_backend_tally_sheets: IPermissions.TALLY_SHEET_CREATE,
    delete_sequent_backend_tally_sheet: IPermissions.TALLY_SHEET_CREATE,
    delete_sequent_backend_tally_sheets: IPermissions.TALLY_SHEET_CREATE,
    // trustee
    sequent_backend_trustee: IPermissions.TRUSTEE_READ,
    sequent_backend_trustees: IPermissions.TRUSTEE_READ,
    insert_sequent_backend_trustee: IPermissions.TRUSTEE_WRITE,
    insert_sequent_backend_trustees: IPermissions.TRUSTEE_WRITE,
    update_sequent_backend_trustee: IPermissions.TRUSTEE_WRITE,
    update_sequent_backend_trustees: IPermissions.TRUSTEE_WRITE,
    delete_sequent_backend_trustee: IPermissions.TRUSTEE_WRITE,
    delete_sequent_backend_trustees: IPermissions.TRUSTEE_WRITE,
}

const TrusteeOperationMap: Record<string, IPermissions> = {
    ...AdminOperationMap,
    // keys_ceremony
    sequent_backend_keys_ceremony: IPermissions.TRUSTEE_CEREMONY,
    sequent_backend_keys_ceremonys: IPermissions.TRUSTEE_CEREMONY,
    // tally_session
    sequent_backend_tally_session: IPermissions.TRUSTEE_CEREMONY,
    sequent_backend_tally_sessions: IPermissions.TRUSTEE_CEREMONY,
    // tally_session_contest
    sequent_backend_tally_session_contest: IPermissions.TRUSTEE_CEREMONY,
    sequent_backend_tally_session_contests: IPermissions.TRUSTEE_CEREMONY,
    // tally_session_execution
    sequent_backend_tally_session_execution: IPermissions.TRUSTEE_CEREMONY,
    sequent_backend_tally_session_executions: IPermissions.TRUSTEE_CEREMONY,
    getUsers: IPermissions.VOTER_READ,
}

export const getOperationRole = (operation: GraphQLRequest, isTrustee = false): IPermissions => {
    let operationName = operation?.operationName
    if (isUndefined(operationName)) {
        return IPermissions.ADMIN_USER
    }
    let OperationMap = isTrustee ? TrusteeOperationMap : AdminOperationMap
    return OperationMap[operationName] ?? IPermissions.ADMIN_USER
}
