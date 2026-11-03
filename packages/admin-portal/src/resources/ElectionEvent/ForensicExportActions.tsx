// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {useApolloClient} from "@apollo/client"
import {
    Button,
    defaultExporter,
    useDataProvider,
    useListContext,
    useNotify,
    useTranslate,
} from "react-admin"
import DownloadIcon from "@mui/icons-material/Download"
import type {IForensicAddress} from "@sequentech/ui-core"
import {ListActions} from "@/components/ListActions"
import {FORENSIC_NETWORK_SUMMARY} from "@/queries/ForensicNetwork"
import {collectForensicExport} from "@/services/ForensicExport"
import {IPermissions} from "@/types/keycloak"

export const ForensicExportActions: React.FC<{
    electionEventId: string
    electionId?: string
    preferenceKey?: string
    selectedOnly?: boolean
}> = ({electionEventId, electionId, preferenceKey, selectedOnly = false}) => {
    const {filterValues, selectedIds, total} = useListContext()
    const client = useApolloClient()
    const dataProvider = useDataProvider()
    const notify = useNotify()
    const translate = useTranslate()
    const [exporting, setExporting] = useState(false)
    const exportRows = async () => {
        setExporting(true)
        try {
            const rows = await collectForensicExport<IForensicAddress>(
                async (offset, limit) => {
                    const {data} = await client.query<{
                        forensic_network_summary: {
                            payload: {items: IForensicAddress[]; total: number}
                        }
                    }>({
                        query: FORENSIC_NETWORK_SUMMARY,
                        variables: {
                            electionEventId,
                            electionId: electionId ?? filterValues.election_id,
                            ip: filterValues.ip,
                            country: filterValues.country,
                            offset,
                            limit,
                            operation: "export",
                        },
                        fetchPolicy: "no-cache",
                        context: {headers: {"x-hasura-role": IPermissions.FORENSIC_READ}},
                    })
                    return data.forensic_network_summary.payload
                },
                selectedOnly ? selectedIds : undefined
            )
            await defaultExporter(rows, async () => ({}), dataProvider, "forensic-network")
        } catch {
            notify("ballotPrivacy.forensic.exportError", {type: "error"})
        } finally {
            setExporting(false)
        }
    }
    if (selectedOnly)
        return selectedIds.length ? (
            <Button
                onClick={() => void exportRows()}
                disabled={exporting}
                label="ra.action.export"
                aria-label={translate("ra.action.export")}
            >
                <DownloadIcon />
            </Button>
        ) : null
    return (
        <ListActions
            withImport={false}
            preferenceKey={preferenceKey}
            doExport={() => void exportRows()}
            isExportDisabled={exporting || !total}
        />
    )
}
