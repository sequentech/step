// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {useNotify, useRecordContext, useRefresh, useUpdate} from "react-admin"
import {Box, Button} from "@mui/material"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {IVR_CONFIG_ANNOTATION} from "@/utils/ivr"
import {DefaultValueFunction, JsonEditor} from "json-edit-react"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"

const RESOURCE = "sequent_backend_election_event"

export const IvrConfig: React.FC = () => {
    const {t} = useTranslation()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const notify = useNotify()
    const refresh = useRefresh()
    const [update] = useUpdate()

    const annotations = (record?.annotations ?? {}) as Record<string, unknown>
    const stringConfig =
        typeof annotations[IVR_CONFIG_ANNOTATION] === "string"
            ? (annotations[IVR_CONFIG_ANNOTATION] as string)
            : "{}"
    const parsedConfig = useMemo(() => {
        try {
            return JSON.parse(stringConfig)
        } catch (e) {
            console.error("Failed to parse the config value as json", e)
            return {}
        }
    }, [stringConfig])

    const [saving, setSaving] = useState(false)
    const [editorData, setEditorData] = useState(parsedConfig)
    const pendingConfigPayload = useMemo(() => {
        return JSON.stringify(editorData)
    }, [editorData])

    const dirty = pendingConfigPayload !== stringConfig
    useEffect(() => {
        setEditorData(parsedConfig)
    }, [parsedConfig])

    if (!record?.id) {
        return null
    }

    const handleDefault: DefaultValueFunction = (input, newKey) => {
        if (input.level === 1 && input.key === "flow") {
            return {phase: "", name: ""}
        }
    }
    const handleCancel = () => {
        setEditorData(parsedConfig)
    }
    const handleSave = () => {
        setSaving(true)
        update(
            RESOURCE,
            {
                id: record.id,
                data: {
                    annotations: {
                        ...annotations,
                        [IVR_CONFIG_ANNOTATION]: pendingConfigPayload,
                    },
                },
                previousData: record,
            },
            {
                onSuccess: () => {
                    setSaving(false)
                    notify(t("electionEventScreen.ivr.common.saveSuccess"), {type: "success"})
                    refresh()
                },
                onError: () => {
                    setSaving(false)
                    notify(t("electionEventScreen.ivr.common.saveError"), {type: "error"})
                },
            }
        )
    }

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <ElectionHeaderStyles.SubTitle>
                {t("electionEventScreen.ivr.config.infoMsg")}
            </ElectionHeaderStyles.SubTitle>
            <JsonEditor
                data={editorData}
                rootName="ivr:config"
                defaultValue={handleDefault}
                maxWidth={"100%"}
                setData={(nextData) => {
                    setEditorData(nextData)
                }}
            />
            <Box sx={{mt: 3, display: "flex", gap: 2}}>
                <Button variant="contained" onClick={handleSave} disabled={!dirty || saving}>
                    {t("common.label.save")}
                </Button>
                <Button variant="contained" onClick={handleCancel} disabled={!dirty || saving}>
                    {t("common.label.cancel")}
                </Button>
            </Box>
        </Box>
    )
}
