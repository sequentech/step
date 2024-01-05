import {useEffect, useState} from "react"
import {useParams} from "react-router-dom"
import {TenantEventType} from ".."

export function useRootBackLink() {
    const [backLink, setBackLink] = useState<string>("")
    const {tenantId, eventId} = useParams<TenantEventType>()

    useEffect(() => {
        if (!tenantId || !eventId) {
            throw new Error("Election Event not found")
        }

        setBackLink(`/tenant/${tenantId}/event/${eventId}`)
    }, [eventId, tenantId])

    return backLink
}
