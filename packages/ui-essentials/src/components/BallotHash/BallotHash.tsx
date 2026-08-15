// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import React, {useEffect, useState} from "react"
import {styled} from "@mui/material/styles"
import IconButton from "../IconButton/IconButton"
import {useTranslation} from "react-i18next"
import {
    faCheck,
    faCircleQuestion,
    faCopy,
    faTriangleExclamation,
} from "@fortawesome/free-solid-svg-icons"
import theme from "../../services/theme"

const HashContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    padding: 10px 22px;
    color: ${({theme}) => theme.palette.green.dark};
    backgroundcolor: ${({theme}) => theme.palette.green.light};
    gap: 8px;
    border-radius: 4px;
    border: 1px solid ${({theme}) => theme.palette.green.dark};
    align-items: center;
    max-width: 700px;
    margin-right: auto;
    margin-left: auto;
`

const BallotHashText = styled(Box)`
    word-break: break-all;
    text-align: center;
    flex: 1;
    min-width: 0;
`

const HashActions = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    flex-shrink: 0;
    margin-left: 8px;
`

const CopyStatus = styled("span")`
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
`

export interface BallotHashCopyLabels {
    copy: string
    copied: string
    error: string
}

export interface BallotHashProps {
    hash: string
    onHelpClick?: () => void
    helpButtonLabel?: string
    copyLabels?: BallotHashCopyLabels
}

type CopyBallotHashStatus = "copied" | "error"
type CopyState = "idle" | CopyBallotHashStatus

const COPY_ICON = {
    idle: faCopy,
    copied: faCheck,
    error: faTriangleExclamation,
}

export const copyBallotHash = async (
    hash: string,
    clipboard: Pick<Clipboard, "writeText"> | undefined
): Promise<CopyBallotHashStatus> => {
    if (!clipboard) {
        return "error"
    }

    try {
        await clipboard.writeText(hash)
        return "copied"
    } catch {
        return "error"
    }
}

const BallotHash: React.FC<BallotHashProps> = ({
    hash,
    onHelpClick,
    helpButtonLabel,
    copyLabels,
}) => {
    const {t} = useTranslation()
    const [copyStatus, setCopyStatus] = useState<CopyState>("idle")

    useEffect(() => setCopyStatus("idle"), [hash])

    useEffect(() => {
        if (copyStatus === "idle") {
            return
        }

        const resetTimeout = window.setTimeout(() => setCopyStatus("idle"), 2000)
        return () => window.clearTimeout(resetTimeout)
    }, [copyStatus])

    const handleCopy = async () => {
        setCopyStatus(await copyBallotHash(hash, navigator.clipboard))
    }

    const copyStatusLabel = copyLabels?.[copyStatus === "idle" ? "copy" : copyStatus]

    return (
        <HashContainer className="hash-container">
            <IconButton
                icon={faCheck}
                sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                fontSize="14px"
            />
            <BallotHashText className="hash-text">
                {t("ballotHash", {ballotId: hash})}
            </BallotHashText>
            <HashActions>
                {copyLabels && hash ? (
                    <IconButton
                        icon={COPY_ICON[copyStatus]}
                        title={copyStatusLabel}
                        sx={{
                            fontSize: "unset",
                            lineHeight: "unset",
                            paddingBottom: "2px",
                            color: theme.palette.customGrey.contrastText,
                        }}
                        fontSize="18px"
                        onClick={handleCopy}
                    />
                ) : null}
                <IconButton
                    icon={faCircleQuestion}
                    title={helpButtonLabel}
                    sx={{
                        fontSize: "unset",
                        lineHeight: "unset",
                        paddingBottom: "2px",
                        color: theme.palette.customGrey.contrastText,
                    }}
                    fontSize="18px"
                    onClick={onHelpClick}
                />
            </HashActions>
            <CopyStatus role="status" aria-live="polite" aria-atomic="true">
                {copyStatus === "idle" ? "" : copyStatusLabel}
            </CopyStatus>
        </HashContainer>
    )
}

export default BallotHash
