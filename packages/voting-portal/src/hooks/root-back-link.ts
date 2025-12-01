// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useState} from "react"
import {useParams} from "react-router-dom"
import {TenantEventType} from ".."

export function useRootBackLink() {
    const [backLink, setBackLink] = useState<string>("")
    const {tenantId, eventId} = useParams<TenantEventType>()

    useEffect(() => {
        if (!tenantId || !eventId) {
            setBackLink(`/`)
        } else {
            setBackLink(`/tenant/${tenantId}/event/${eventId}`)
        }
    }, [eventId, tenantId])

    return backLink
}
