// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect} from "react"
import {useApolloClient} from "@apollo/client"
import {useListContext, useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {
    APPLICATION_IDS_PAGE,
    GET_APPLICATION_IDS,
    IGetApplicationIdsResult,
} from "@/queries/GetApplicationIds"
import {
    ISupportReferenceMatch,
    SUPPORT_REFERENCE_FILTER,
    findBySupportReference,
    normalizeSupportReference,
} from "./supportReference"

/**
 * When the queue's search reads as a support reference (the one the pending
 * page gives the voter), finds the applications it belongs to and adds them
 * to the search's matches. The reference can't be turned back into an id, so
 * the ids of the queue's applications (`scope`: the event and Post) are read
 * and their references compared, in the browser.
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
    const scopeKey = JSON.stringify(scope)

    useEffect(() => {
        if (!reference || resolved === reference) return
        let cancelled = false
        const where = {
            _and: Object.entries(scope)
                .filter(([, value]) => value !== undefined && value !== null)
                .map(([key, value]) => ({[key]: {_eq: value}})),
        }
        const resolve = async () => {
            const ids: string[] = []
            for (let offset = 0; ; offset += APPLICATION_IDS_PAGE) {
                const {data} = await client.query<IGetApplicationIdsResult>({
                    query: GET_APPLICATION_IDS,
                    variables: {where, offset},
                    fetchPolicy: "network-only",
                })
                const page = data?.sequent_backend_applications ?? []
                ids.push(...page.map((application) => application.id))
                if (page.length < APPLICATION_IDS_PAGE) break
            }
            return findBySupportReference(ids, reference)
        }
        resolve()
            .then((matches) => {
                if (cancelled) return
                const match: ISupportReferenceMatch = {reference, ids: matches}
                setFilters({...filterValues, [SUPPORT_REFERENCE_FILTER]: match}, {})
            })
            .catch(() => {
                if (!cancelled) {
                    notify(t("approvalsScreen.list.referenceError"), {type: "error"})
                }
            })
        return () => {
            cancelled = true
        }
    }, [reference, resolved, scopeKey])
}
