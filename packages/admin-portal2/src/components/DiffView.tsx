// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useEffect, useMemo, useState} from "react"
import {styled} from "@mui/material/styles"
import {diffLines} from "diff"
import {CircularProgress} from "@mui/material"
import {useTranslation} from "react-i18next"
import {convertToNumber} from "@/lib/helpers"
import {Button} from "react-admin"
import {Dialog} from "@sequentech/ui-essentials"
import {debounce} from "lodash"

const DiffViewStyled = {
    Header: styled("span")`
        font-family: Roboto;
        font-size: 14px;
        font-weight: 500;
        line-height: 24px;
        letter-spacing: 0.4000000059604645px;
        text-align: left;
        text-transform: uppercase;
    `,
    Container: styled("div")`
        gap: 16px;
        display: flex;
        justify-content: space-around;
    `,
    Content: styled("div")`
        gap: 12px;
        display: flex;
        flex-direction: column;
        width: 100%;
    `,
    Block: styled("div")`
        position: relative;
        display: flex;
        flex-direction: column;
        gap: 1rem;
        background-color: #f5f5f5;
        padding: 16px;
        height: 100%;
        width: 100%;
        max-height: 500px;
    `,
    Json: styled("div")`
        width: 100%;
        max-width: 35vw;
        overflow-x: scroll;
    `,
    Removed: styled("pre")`
        width: 100%;
        font-size: 12px;
        background-color: #fa958e;
        text-decoration: line-through;
    `,
    Added: styled("pre")`
        width: 100%;
        font-size: 12px;
        background-color: #43e3a1;
    `,
    Line: styled("pre")`
        width: 100%;
        font-size: 12px;
    `,
    Loading: styled("div")`
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
    fetchAllPublishChanges: () => Promise<void>
}

enum TRUNCATION_STATE {
    NOT_NEEDED = 0,
    TRUNCATED = 1,
    UNTRUNCATED = 2,
}

// Truncate the strings if they are too long
const truncateLines = (str: string, maxLines: number) => {
    const lines = str.split("\n")
    if (lines.length > maxLines) {
        return lines.slice(0, maxLines).join("\n") + "\n..."
    }
    return str
}

const DiffViewMemo = React.memo(
    <T extends {}>({
        current,
        currentTitle,
        modify,
        diffTitle,
        type = "modify",
        fetchAllPublishChanges,
    }: TDiffView<T>) => {
        const MAX_DIFF_LINES = convertToNumber(process.env.MAX_DIFF_LINES) ?? 500
        const [diff, setDiff] = useState<any>("")
        const {t} = useTranslation()
        const [oldJsonString, setOldJsonString] = useState<string>("")
        const [newJsonString, setNewJsonString] = useState<string>("")
        const [showDialog, setShowDialog] = useState<boolean>(false)
        const [loading, setLoading] = useState<boolean>(false)

        const [truncationState, setTruncationState] = useState<TRUNCATION_STATE>(
            TRUNCATION_STATE.NOT_NEEDED
        )

        const memoizedModify = useMemo(
            () => (modify ? JSON.stringify(modify, null, 2) : ""),
            [modify]
        )
        const memoizedCurrent = useMemo(
            () => (current ? JSON.stringify(current, null, 2) : ""),
            [current]
        )
        useEffect(() => {
            if (!memoizedModify || truncationState !== TRUNCATION_STATE.NOT_NEEDED) return
            const lines = memoizedModify.split("\n")
            if (lines.length < MAX_DIFF_LINES) return
            setTruncationState(TRUNCATION_STATE.TRUNCATED)
        }, [memoizedModify, MAX_DIFF_LINES, truncationState])

        useEffect(() => {
            if (truncationState === TRUNCATION_STATE.TRUNCATED) {
                setNewJsonString(truncateLines(memoizedModify, MAX_DIFF_LINES))
                setOldJsonString(truncateLines(memoizedCurrent, MAX_DIFF_LINES))
            } else {
                setNewJsonString(memoizedModify)
                setOldJsonString(memoizedCurrent)
            }
        }, [truncationState, memoizedCurrent, memoizedModify, MAX_DIFF_LINES])

        const calculateDiff = useCallback(
            debounce(() => {
                if (newJsonString || oldJsonString) {
                    const diffText: any = diffLines(oldJsonString, newJsonString)
                    setDiff(diffText)
                }
            }, 100),
            [oldJsonString, newJsonString]
        )

        useEffect(() => {
            calculateDiff()
            return () => {
                calculateDiff.cancel()
            }
        }, [calculateDiff])

        const handleDialogClose = useCallback(
            async (result: boolean) => {
                if (result) {
                    let shouldUpdateData = false
                    setTruncationState((prev) => {
                        if (prev === TRUNCATION_STATE.TRUNCATED) {
                            shouldUpdateData = true
                            return TRUNCATION_STATE.UNTRUNCATED
                        }
                        return TRUNCATION_STATE.TRUNCATED
                    })
                    if (shouldUpdateData) {
                        setLoading(true)
                        await fetchAllPublishChanges()
                        setLoading(false)
                    }
                }
                setShowDialog(false)
            },
            [fetchAllPublishChanges]
        )

        if (!diff) {
            return (
                <DiffViewStyled.Loading>
                    <CircularProgress />
                </DiffViewStyled.Loading>
            )
        }

        return (
            <>
                <DiffViewStyled.Container>
                    <DiffViewStyled.Content>
                        <DiffViewStyled.Header>{currentTitle}</DiffViewStyled.Header>
                        <DiffViewStyled.Block>
                            <DiffViewStyled.Json>
                                {diff?.map((line: any, index: number) =>
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
                            {truncationState !== TRUNCATION_STATE.NOT_NEEDED && (
                                <Button
                                    onClick={() => {
                                        if (truncationState === TRUNCATION_STATE.UNTRUNCATED) {
                                            setTruncationState(TRUNCATION_STATE.TRUNCATED)
                                        } else {
                                            setShowDialog(true)
                                        }
                                    }}
                                    label={
                                        truncationState === TRUNCATION_STATE.TRUNCATED
                                            ? t("electionEventScreen.common.showMore")
                                            : t("electionEventScreen.common.showLess")
                                    }
                                    style={{
                                        color: "#fff",
                                        width: "fit-content",
                                        minHeight: "unset",
                                        fontSize: "0.8rem",
                                        position: "absolute",
                                        right: "0.5rem",
                                        bottom: "0.5rem",
                                    }}
                                    aria-expanded={truncationState !== TRUNCATION_STATE.TRUNCATED}
                                    aria-controls="diff-content"
                                />
                            )}
                        </DiffViewStyled.Block>
                    </DiffViewStyled.Content>

                    {type === "modify" && (
                        <DiffViewStyled.Content>
                            <DiffViewStyled.Header>{diffTitle}</DiffViewStyled.Header>
                            <DiffViewStyled.Block>
                                <DiffViewStyled.Json>
                                    {diff?.map((line: any, index: number) =>
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
                                {truncationState !== TRUNCATION_STATE.NOT_NEEDED && (
                                    <Button
                                        onClick={() => {
                                            if (truncationState === TRUNCATION_STATE.UNTRUNCATED) {
                                                setTruncationState(TRUNCATION_STATE.TRUNCATED)
                                            } else {
                                                setShowDialog(true)
                                            }
                                        }}
                                        label={
                                            truncationState === TRUNCATION_STATE.TRUNCATED
                                                ? t("electionEventScreen.common.showMore")
                                                : t("electionEventScreen.common.showLess")
                                        }
                                        style={{
                                            color: "#fff",
                                            width: "fit-content",
                                            minHeight: "unset",
                                            fontSize: "0.8rem",
                                            position: "absolute",
                                            right: "0.5rem",
                                            bottom: "0.5rem",
                                        }}
                                        aria-expanded={
                                            truncationState !== TRUNCATION_STATE.TRUNCATED
                                        }
                                        aria-controls="diff-content"
                                    />
                                )}
                            </DiffViewStyled.Block>
                        </DiffViewStyled.Content>
                    )}
                </DiffViewStyled.Container>
                <Dialog
                    variant="warning"
                    open={showDialog}
                    ok={String(t("publish.dialog.ok"))}
                    cancel={String(t("publish.dialog.ko"))}
                    title={String(t("publish.dialog.title"))}
                    handleClose={handleDialogClose}
                    okEnabled={() => !loading}
                >
                    <>
                        <DiffViewStyled.Content>
                            {t("publish.dialog.diff")}
                            {loading && <CircularProgress />}
                        </DiffViewStyled.Content>
                    </>
                </Dialog>
            </>
        )
    }
)

DiffViewMemo.displayName = "DiffView"

export const DiffView = DiffViewMemo
