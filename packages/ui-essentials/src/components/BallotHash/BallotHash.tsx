// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import React from "react"
import {styled} from "@mui/material/styles"
import IconButton from "../IconButton/IconButton"
import {useTranslation} from "react-i18next"
import {faCircleQuestion, faCheck} from "@fortawesome/free-solid-svg-icons"
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
`

export interface BallotHashProps {
    hash: string
    onHelpClick?: () => void
}

const BallotHash: React.FC<BallotHashProps> = ({hash, onHelpClick}) => {
    const {t} = useTranslation()

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
            <IconButton
                icon={faCircleQuestion}
                sx={{
                    fontSize: "unset",
                    lineHeight: "unset",
                    paddingBottom: "2px",
                    marginLeft: "16px",
                    color: theme.palette.customGrey.contrastText,
                }}
                fontSize="18px"
                onClick={onHelpClick}
            />
        </HashContainer>
    )
}

export default BallotHash
