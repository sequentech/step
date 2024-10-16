// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"

import styled from "@emotion/styled"

import {Box} from "@mui/material"
import {Button} from "react-admin"
import {useTranslation} from "react-i18next"
import {Publish} from "@mui/icons-material"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"

import {DiffView} from "@/components/DiffView"
import {PublishActions} from "./PublishActions"
import {EPublishActionsType} from "./EPublishType"
import {PublishStatus} from "./EPublishStatus"
import {CancelButton} from "../Tally/styles"

const PublishGenerateStyled = {
    WizardContainer: styled.div`
        display: flex;
        flex-direction: column;
        min-height: 100%;
    `,
    ContentWrapper: styled.div`
        flex-grow: 1;
        overflow-y: auto;
        padding-bottom: 1rem; // Add some padding at the bottom to prevent content from being hidden behind the footer
    `,
    FooterContainer: styled.div`
        width: 100%;
        position: sticky;
        bottom: 0;
        background-color: ${({theme}) => theme.palette.background.default};
        box-shadow: 0 -2px 4px rgba(0, 0, 0, 0.1);
    `,
    StyledFooter: styled.div`
        margin: 0 auto;
        display: flex;
        justify-content: space-between;
        padding: 1rem;
    `,
    Container: styled.div`
        display: flex;
        flex-direction: column;
        gap: 32px;
        margin-top: -12px;
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
        display: flex;
        padding: 8px 16px;
        width: 100%;
        background-color: #f5f5f5;
        justify-content: space-between;
    `,
}

export type TPublishGenerate = {
    data: any
    readOnly: boolean
    status: PublishStatus
    changingStatus: boolean
    electionId?: string
    onBack: () => void
    onPublish: () => void
    onGenerate: () => void
    electionEventId: string
    fetchAllPublishChanges: () => Promise<void>
}

export const PublishGenerate: React.FC<TPublishGenerate> = ({
    data,
    status,
    changingStatus,
    readOnly,
    onBack = () => null,
    onPublish = () => null,
    onGenerate = () => null,
    fetchAllPublishChanges,
}): React.JSX.Element => {
    const {t} = useTranslation()

    return (
        <PublishGenerateStyled.WizardContainer>
            <PublishGenerateStyled.ContentWrapper>
                {!readOnly && (
                    <PublishActions
                        status={status}
                        changingStatus={changingStatus}
                        onPublish={onPublish}
                        onGenerate={onGenerate}
                        type={EPublishActionsType.Generate}
                    />
                )}

                <PublishGenerateStyled.AccordionHeaderTitle>
                    {readOnly ? t("publish.header.viewChange") : t("publish.header.change")}
                </PublishGenerateStyled.AccordionHeaderTitle>

                <DiffView
                    currentTitle={
                        readOnly ? t("publish.label.previous") : t("publish.label.current")
                    }
                    diffTitle={readOnly ? t("publish.label.publication") : t("publish.label.diff")}
                    current={data?.previous || null}
                    modify={data?.current || null}
                    fetchAllPublishChanges={fetchAllPublishChanges}
                />
            </PublishGenerateStyled.ContentWrapper>

            <PublishGenerateStyled.FooterContainer>
                <PublishGenerateStyled.StyledFooter>
                    <CancelButton onClick={onBack} className="list-actions">
                        <ArrowBackIosIcon />
                        {t("common.label.back")}
                    </CancelButton>

                    {!readOnly && (
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
                </PublishGenerateStyled.StyledFooter>
            </PublishGenerateStyled.FooterContainer>
        </PublishGenerateStyled.WizardContainer>
    )
}
