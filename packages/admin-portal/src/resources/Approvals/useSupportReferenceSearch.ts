// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect} from "react"
import {useApolloClient} from "@apollo/client"
import {useListContext, useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {
    FIND_APPLICATIONS_BY_SUPPORT_REFERENCE,
    IFindApplicationsBySupportReferenceResult,
} from "@/queries/FindApplicationsBySupportReference"
import {IPermissions} from "@/types/keycloak"
import {
    ISupportReferenceMatch,
    SUPPORT_REFERENCE_FILTER,
    normalizeSupportReference,
} from "./supportReference"

/**
 * How long the search must stay the same before a reference is looked up: the
 * server hashes the event's applications for each lookup, so not one per
 * keystroke.
 */
export const SUPPORT_REFERENCE_DEBOUNCE_MS = 400

/**
 * When the queue's search reads as a support reference (the one the pending
 * page gives the voter), asks the server for the applications it belongs to
 * within the queue's scope (`scope`: the event and its label) and adds them
 * to the search's matches. The list then reads them with the viewer's own
 * permissions, so the search shows nothing the queue wouldn't. It asks once
 * the search has stayed the same for [`SUPPORT_REFERENCE_DEBOUNCE_MS`].
 */
export const useSupportReferenceSearch = (scope: Record<string, unknown>) => {
    const {filterValues, setFilters} = useListContext()
    const client = useApolloClient()
    const notify = useNotify()
    const {t} = useTranslation()
    const search = typeof filterValues?.q === "string" ? filterValues.q : ""
    const reference = normalizeSupportReference(search)
    const resolved = (
        filterValues?.[SUPPORT_REFERENCE_FILTER] as ISupportReferenceMatch | undefined
    )?.reference
    const electionEventId =
        typeof scope.election_event_id === "string" ? scope.election_event_id : undefined
    const permissionLabel =
        typeof scope.permission_label === "string" ? scope.permission_label : undefined

    useEffect(() => {
        if (!reference || resolved === reference || !electionEventId) return
        let cancelled = false
        const lookUp = () =>
            client
                .query<IFindApplicationsBySupportReferenceResult>({
                    query: FIND_APPLICATIONS_BY_SUPPORT_REFERENCE,
                    variables: {electionEventId, reference, permissionLabel},
                    fetchPolicy: "network-only",
                    context: {headers: {"x-hasura-role": IPermissions.APPLICATION_READ}},
                })
                .then(({data}) => {
                    if (cancelled) return
                    const match: ISupportReferenceMatch = {
                        reference,
                        ids: data?.find_applications_by_support_reference?.application_ids ?? [],
                    }
                    setFilters({...filterValues, [SUPPORT_REFERENCE_FILTER]: match}, {})
                })
                .catch(() => {
                    if (!cancelled) {
                        notify(t("approvalsScreen.list.referenceError"), {type: "error"})
                    }
                })
        const timer = setTimeout(lookUp, SUPPORT_REFERENCE_DEBOUNCE_MS)
        return () => {
            cancelled = true
            clearTimeout(timer)
        }
    }, [reference, resolved, electionEventId, permissionLabel])
}
