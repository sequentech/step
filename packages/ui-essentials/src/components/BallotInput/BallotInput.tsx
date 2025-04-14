// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState, useEffect} from "react"
import {useTranslation} from "react-i18next"
import {theme, Icon, IconButton, Dialog} from "../../index"
import {stringToHtml} from "@sequentech/ui-core"
import {Box, TextField, Typography, Button, InputLabelProps} from "@mui/material"
import {styled} from "@mui/material/styles"
import {Link, useLocation, useParams} from "react-router-dom"
import {faAngleLeft, faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {IBallotStyle as IElectionDTO} from "@sequentech/ui-core"

const StyledLink = styled(Link)`
    text-decoration: none;
`

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    font-size: 24px;
    font-weight: 500;
    line-height: 27px;
    margin-top: 20px;
    margin-bottom: 16px;
`

const StyledError = styled(Typography)`
    position: absolute;
    margin-top: -12px;
    color: ${({theme}) => theme.palette.red.main};
`

function isHex(str: string) {
    if (str.trim() === "") {
        return true
    }

    const regex = /^[0-9a-fA-F]+$/
    return regex.test(str)
}

export interface IBallotStyle {
    id: string
    election_id: string
    election_event_id: string
    tenant_id: string
    ballot_eml: IElectionDTO
    ballot_signature?: string | null
    created_at: string
    area_id?: string | null
    annotations?: string | null
    labels?: string | null
    last_updated_at: string
}

interface IBallotInputProps {
    title: string
    subTitle: string
    label?: string
    error: string
    placeholder: string
    value: string
    doChange: (event: React.ChangeEvent<HTMLInputElement>) => void
    captureEnterAction: React.KeyboardEventHandler<HTMLDivElement>
    labelProps: InputLabelProps
    helpText: string
    dialogTitle: string
    dialogOk: string
    backButtonText?: string
    ballotStyle?: IBallotStyle | undefined
}

const BallotInput: React.FC<IBallotInputProps> = ({
    title,
    subTitle,
    label = "",
    error = "",
    placeholder = "",
    value,
    doChange,
    captureEnterAction,
    labelProps,
    helpText,
    dialogTitle,
    dialogOk,
    backButtonText = "",
    ballotStyle,
}) => {
    const {t, i18n} = useTranslation()
    const {tenantId, eventId, electionId} = useParams()

    const [openTitleHelp, setOpenTitleHelp] = useState<boolean>(false)
    const location = useLocation()
    const [inputBallotId, setInputBallotId] = useState<string>("")

    useEffect(() => {
        const currLanguage = i18n.language
        const electionLanguages =
            ballotStyle?.ballot_eml?.election_presentation?.language_conf?.enabled_language_codes
        const defaultLang =
            ballotStyle?.ballot_eml?.election_presentation?.language_conf?.default_language_code
        if (
            !electionLanguages ||
            !currLanguage ||
            electionLanguages.includes(currLanguage) ||
            !defaultLang
        )
            return
        i18n.changeLanguage(defaultLang)
    }, [ballotStyle])

    const validatedBallotId = isHex(value ?? "")

    return (
        <>
            <Box
                sx={{
                    display: "flex",
                    flexDirection: {xs: "column", md: "row"},
                    justifyContent: "space-between",
                    alignItems: "flex-start",
                }}
            >
                <Box
                    sx={{
                        order: {xs: 2, md: 1},
                    }}
                >
                    <StyledTitle variant="h1">
                        <Box>{t(title)}</Box>
                        <IconButton
                            icon={faCircleQuestion}
                            sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                            fontSize="16px"
                            onClick={() => setOpenTitleHelp(true)}
                        />
                        <Dialog
                            handleClose={() => setOpenTitleHelp(false)}
                            open={openTitleHelp}
                            title={t(dialogTitle)}
                            ok={t(dialogOk)}
                            variant="info"
                        >
                            {stringToHtml(t(helpText))}
                        </Dialog>
                    </StyledTitle>

                    <Typography variant="body1" sx={{color: theme.palette.customGrey.contrastText}}>
                        {t(subTitle)}
                    </Typography>
                </Box>
                <Box sx={{order: {xs: 1, md: 2}, marginTop: "20px"}}>
                    {backButtonText ? (
                        <StyledLink
                            to={`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`}
                        >
                            <Button variant="secondary" className="secondary">
                                <Icon icon={faAngleLeft} size="sm" />
                                <Box paddingLeft="12px">{t(backButtonText)}</Box>
                            </Button>
                        </StyledLink>
                    ) : null}
                </Box>
            </Box>

            <>
                <TextField
                    onChange={doChange}
                    value={value}
                    InputLabelProps={labelProps}
                    label={t(label)}
                    placeholder={t(placeholder)}
                    onKeyDown={captureEnterAction}
                />
                {!validatedBallotId && <StyledError>{t(error)}</StyledError>}
            </>
        </>
    )
}

export default BallotInput
