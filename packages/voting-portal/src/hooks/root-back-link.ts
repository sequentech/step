import {useEffect, useState} from "react"

export function useRootBackLink() {
    const [backLink, setBackLink] = useState<string>("")

    useEffect(() => {
        let tenantEvent

        try {
            tenantEvent = JSON.parse(localStorage.getItem("tenant-event") ?? "")
        } catch (e) {
            console.warn(e)
        }

        const backLink =
            tenantEvent?.eventId && tenantEvent?.tenantId
                ? `/tenant/${tenantEvent.tenantId}/event/${tenantEvent.eventId}/election-chooser`
                : "/"

        setBackLink(backLink)
    }, [])

    return backLink
}
