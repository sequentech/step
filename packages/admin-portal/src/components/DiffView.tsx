// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"

import styled from "@emotion/styled"

import {diffLines} from "diff"
import {CircularProgress} from "@mui/material"
import {useTranslation} from "react-i18next"

const DiffViewStyled = {
    Header: styled.span`
        font-family: Roboto;
        font-size: 14px;
        font-weight: 500;
        line-height: 24px;
        letter-spacing: 0.4000000059604645px;
        text-align: left;
        text-transform: uppercase;
    `,
    Container: styled.div`
        gap: 16px;
        display: flex;
        justify-content: space-around;
    `,
    Content: styled.div`
        gap: 12px;
        display: flex;
        flex-direction: column;
        width: 100%;
    `,
    Block: styled.div`
        display: flex;
        background-color: #f5f5f5;
        padding: 16px;
        height: 100%;
        width: 100%;
        overflow-x: scroll;
        max-height: 500px;
    `,
    Json: styled.div`
        width: 100%;
        max-width: 35vw;
        overflow-x: scroll;
    `,
    Removed: styled.pre`
        width: 100%;
        font-size: 12px;
        background-color: #fa958e;
        text-decoration: line-through;
    `,
    Added: styled.pre`
        width: 100%;
        font-size: 12px;
        background-color: #43e3a1;
    `,
    Line: styled.pre`
        width: 100%;
        font-size: 12px;
    `,
    Loading: styled.div`
        display: flex;
        height: 60vh;
        justify-content: center;
        align-items: center;
    `,
}

type TDiffView<T> = {
    type?: "simple" | "modify"
    diffTitle: string
    currentTitle: string
    current: T
    modify: T
}

// TODO: Make this configurable
const MAX_DIFF_LINES = 500

// Truncate the strings if they are too long
const truncateLines = (str: string, maxLines: number) => {
    const lines = str.split("\n")
    if (lines.length > maxLines) {
        return lines.slice(0, maxLines).join("\n") + "\n..."
    }
    return str
}

const DiffViewMemo = React.memo(
    <T extends {}>({current, currentTitle, modify, diffTitle, type = "modify"}: TDiffView<T>) => {
        const [diff, setDiff] = useState<any>("")
        const {t} = useTranslation()
        const [oldJsonString, setOldJsonString] = useState<string>("")
        const [newJsonString, setNewJsonString] = useState<string>("")

        useEffect(() => {
            setNewJsonString(truncateLines(JSON.stringify(modify, null, 2), MAX_DIFF_LINES))
            setOldJsonString(truncateLines(JSON.stringify(current, null, 2), MAX_DIFF_LINES))
        }, [modify, current])

        useEffect(() => {
            if (oldJsonString && newJsonString) {
                const diffText: any = diffLines(oldJsonString, newJsonString)

                console.log(diffText)

                setDiff(diffText)
            }
        }, [oldJsonString, newJsonString])

        if (!diff) {
            return (
                <DiffViewStyled.Loading>
                    <CircularProgress />
                </DiffViewStyled.Loading>
            )
        }

        return (
            <DiffViewStyled.Container>
                <DiffViewStyled.Content>
                    <DiffViewStyled.Header>{currentTitle}</DiffViewStyled.Header>
                    <DiffViewStyled.Block>
                        <DiffViewStyled.Json>
                            {diff.map((line: any, index: number) =>
                                !line.added ? (
                                    line.removed && type === "modify" ? (
                                        <DiffViewStyled.Removed key={index}>
                                            {line.value}
                                        </DiffViewStyled.Removed>
                                    ) : (
                                        <DiffViewStyled.Line key={index}>
                                            {line.value === "null"
                                                ? t("common.label.loadingData")
                                                : line.value}
                                        </DiffViewStyled.Line>
                                    )
                                ) : null
                            )}
                        </DiffViewStyled.Json>
                    </DiffViewStyled.Block>
                </DiffViewStyled.Content>

                {type === "modify" && (
                    <DiffViewStyled.Content>
                        <DiffViewStyled.Header>{diffTitle}</DiffViewStyled.Header>
                        <DiffViewStyled.Block>
                            <DiffViewStyled.Json>
                                {diff.map((line: any, index: number) =>
                                    !line.removed ? (
                                        line.added ? (
                                            <DiffViewStyled.Added key={index}>
                                                {line.value}
                                            </DiffViewStyled.Added>
                                        ) : (
                                            <DiffViewStyled.Line key={index}>
                                                {line.value === "null"
                                                    ? t("common.label.loadingData")
                                                    : line.value}
                                            </DiffViewStyled.Line>
                                        )
                                    ) : null
                                )}
                            </DiffViewStyled.Json>
                        </DiffViewStyled.Block>
                    </DiffViewStyled.Content>
                )}
            </DiffViewStyled.Container>
        )
    }
)

DiffViewMemo.displayName = "DiffView"

export const DiffView = DiffViewMemo
