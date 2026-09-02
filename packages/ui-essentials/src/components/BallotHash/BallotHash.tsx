// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import React, {useEffect, useState} from "react"
import {styled} from "@mui/material/styles"
import IconButton from "../IconButton/IconButton"
import Icon from "../Icon/Icon"
import DecorativeIconBox from "../Icon/DecorativeIconBox"
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

export enum CopyBallotHashStatus {
    Idle = "idle",
    Copied = "copied",
    Error = "error",
}

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
        return CopyBallotHashStatus.Error
    }

    try {
        await clipboard.writeText(hash)
        return CopyBallotHashStatus.Copied
    } catch {
        return CopyBallotHashStatus.Error
    }
}

const BallotHash: React.FC<BallotHashProps> = ({
    hash,
    onHelpClick,
    helpButtonLabel,
    copyLabels,
}) => {
    const {t} = useTranslation()
    const [copyStatus, setCopyStatus] = useState(CopyBallotHashStatus.Idle)

    useEffect(() => setCopyStatus(CopyBallotHashStatus.Idle), [hash])

    useEffect(() => {
        if (copyStatus === CopyBallotHashStatus.Idle) {
            return
        }

        const resetTimeout = window.setTimeout(() => setCopyStatus(CopyBallotHashStatus.Idle), 2000)
        return () => window.clearTimeout(resetTimeout)
    }, [copyStatus])

    const handleCopy = async () => {
        setCopyStatus(await copyBallotHash(hash, navigator.clipboard))
    }

    const copyStatusLabel =
        copyLabels?.[copyStatus === CopyBallotHashStatus.Idle ? "copy" : copyStatus]

    return (
        <HashContainer className="hash-container">
            <DecorativeIconBox className="hash-check">
                <Icon
                    icon={faCheck}
                    style={{fontSize: "14px", lineHeight: "unset", paddingBottom: "2px"}}
                />
            </DecorativeIconBox>
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
                    ariaLabel={helpButtonLabel || t("a11y.ballotIdHelp")}
                />
            </HashActions>
            <CopyStatus role="status" aria-live="polite" aria-atomic="true">
                {copyStatus === CopyBallotHashStatus.Idle ? "" : copyStatusLabel}
            </CopyStatus>
        </HashContainer>
    )
}

export default BallotHash
