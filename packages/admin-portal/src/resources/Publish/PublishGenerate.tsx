// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"

import styled from "@emotion/styled"

import {Box} from "@mui/material"
import {Button, Identifier, useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {ArrowBackIosNew, Publish} from "@mui/icons-material"
import {Preview} from "@mui/icons-material"

import {DiffView} from "@/components/DiffView"
import {PublishActions} from "./PublishActions"
import {EPublishActionsType, EPublishType} from "./EPublishType"
import {PublishStatus} from "./EPublishStatus"
import {usePublishPermissions} from "./usePublishPermissions"
import PublishExport from "./PublishExport"

const PublishGenerateStyled = {
    Container: styled.div`
        display: flex;
        flex-direction: column;
        gap: 32px;
        margin-top: -12px;
    `,
    TitleWrapper: styled.div`
        display: flex;
        flex-direction: row;
        justify-content: space-between;
    `,
    AccordionHeaderTitle: styled.span`
        font-family: Roboto;
        font-size: 24px;
        font-weight: 700;
        line-height: 32px;
        letter-spacing: 0px;
        text-align: left;
    `,
    Loading: styled.div`
        display: flex;
        height: 60vh;
        justify-content: center;
        align-items: center;
    `,
    Bottom: styled.div`
        position: sticky;
        bottom: 0;
        display: flex;
        padding: 8px 16px;
        width: 100%;
        background-color: #f5f5f5;
        justify-content: space-between;
    `,
}

export type TPublishGenerate = {
    ballotPublicationId?: string | Identifier | null
    data: any
    publishType: EPublishType.Election | EPublishType.Event
    readOnly: boolean
    status: PublishStatus
    changingStatus: boolean
    electionId?: string
    onBack: () => void
    onPublish: () => void
    onGenerate: () => void
    electionEventId: string
    fetchAllPublishChanges: () => Promise<void>
    onPreview: (id: string | Identifier) => void
}

export const PublishGenerate: React.FC<TPublishGenerate> = ({
    ballotPublicationId,
    publishType,
    data,
    status,
    changingStatus,
    readOnly,
    onBack = () => null,
    onPublish = () => null,
    onGenerate = () => null,
    fetchAllPublishChanges,
    onPreview = () => null,
}): React.JSX.Element => {
    const {t} = useTranslation()
    const notify = useNotify()

    const {
        canReadPublish,
        canWritePublish,
        canPublishCreate,
        canPublishRegenerate,
        canPublishExport,
        canPublishStartVoting,
        canPublishPauseVoting,
        canPublishStopVoting,
        canPublishChanges,
        showPublishPreview,
        showPublishView,
        showPublishButtonBack,
        showPublishColumns,
        showPublishFilters,
    } = usePublishPermissions()

    const onPreviewClick = () => {
        if (ballotPublicationId) {
            onPreview(ballotPublicationId)
        } else {
            notify(t("publish.dialog.error_preview"), {
                type: "error",
            })
        }
    }

    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            {!readOnly && (
                <PublishActions
                    ballotPublicationId={ballotPublicationId}
                    status={status}
                    publishType={publishType}
                    electionStatus={null}
                    electionPresentation={null}
                    kioskModeEnabled={false}
                    changingStatus={changingStatus}
                    onPublish={onPublish}
                    onGenerate={onGenerate}
                    data={data}
                    type={EPublishActionsType.Generate}
                />
            )}

            <PublishGenerateStyled.Container>
                <PublishGenerateStyled.TitleWrapper>
                    <PublishGenerateStyled.AccordionHeaderTitle>
                        {readOnly ? t("publish.header.viewChange") : t("publish.header.change")}
                    </PublishGenerateStyled.AccordionHeaderTitle>
                    {readOnly && <PublishExport ballotPublicationId={ballotPublicationId} />}
                </PublishGenerateStyled.TitleWrapper>

                <DiffView
                    currentTitle={
                        readOnly ? t("publish.label.previous") : t("publish.label.current")
                    }
                    diffTitle={readOnly ? t("publish.label.publication") : t("publish.label.diff")}
                    current={data?.previous || null}
                    modify={data?.current || null}
                    fetchAllPublishChanges={fetchAllPublishChanges}
                />

                <PublishGenerateStyled.Bottom>
                    {/* Left container for the back button */}
                    <div>
                        {showPublishButtonBack ? (
                            <Button
                                onClick={onBack}
                                label={t("publish.action.back")}
                                className="publish-back-button"
                                style={{
                                    backgroundColor: "#eee",
                                    color: "#0f054c",
                                }}
                            >
                                <ArrowBackIosNew />
                            </Button>
                        ) : null}
                    </div>

                    {/* Right container for the preview and publish buttons */}
                    <div style={{display: "flex", gap: "8px"}}>
                        {showPublishPreview && showPublishView ? (
                            <Button
                                onClick={onPreviewClick}
                                label={t("publish.preview.action")}
                                className="publish-preview-button"
                            >
                                <Preview />
                            </Button>
                        ) : null}

                        {!readOnly && canWritePublish && (
                            <Button
                                onClick={onPublish}
                                label={t("publish.action.publish")}
                                className="publish-publish-button"
                                style={{
                                    color: "#fff",
                                }}
                            >
                                <Publish />
                            </Button>
                        )}
                    </div>
                </PublishGenerateStyled.Bottom>
            </PublishGenerateStyled.Container>
        </Box>
    )
}
