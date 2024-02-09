import {useContext} from "react"
import {useParams} from "react-router-dom"
import {TenantEventType} from ".."
import {SettingsContext} from "../providers/SettingsContextProvider"

export function useGetPublicDocumentUrl(documentId: string) {
    const {tenantId} = useParams<TenantEventType>()
    const {globalSettings} = useContext(SettingsContext)

    function getDocumentUrl(documentName: string): string {
        return encodeURI(
            `${globalSettings.PUBLIC_BUCKET_URL}tenant-${tenantId}/document-${documentId}/${documentName}`
        )
    }

    return {
        getDocumentUrl,
    }
}
