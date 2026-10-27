// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Button} from "@mui/material"
import {useRecordContext} from "react-admin"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {usePublishPermissions} from "./usePublishPermissions"
import type {Sequent_Backend_Ballot_Publication} from "@/gql/graphql"

/** Reopen an interrupted review; publication still uses the ordinary signed action. */
export const ResumePublication = ({
    electionEventId,
    onResume,
}: {
    electionEventId: string | number | undefined
    onResume?: (id: string) => void
}) => {
    const record = useRecordContext<Sequent_Backend_Ballot_Publication>()
    const [tenantId] = useTenantStore()
    const {canReadPublish, canWritePublish} = usePublishPermissions()
    const {t} = useTranslation()
    if (
        !onResume ||
        !canReadPublish ||
        !canWritePublish ||
        !record?.id ||
        record.tenant_id !== tenantId ||
        record.election_event_id !== electionEventId ||
        record.is_generated !== true ||
        record.published_at !== null ||
        record.deleted_at !== null
    ) {
        return null
    }
    return (
        <Button onClick={() => onResume(record.id)} className="publish-resume-button">
            {t("publish.action.resumeApproval", {defaultValue: "Resume approval"})}
        </Button>
    )
}
