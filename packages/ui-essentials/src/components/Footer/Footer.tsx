// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {styled} from "@mui/material/styles"
import Link from "@mui/material/Link"
import Typography from "@mui/material/Typography"
import Paper, {PaperProps} from "@mui/material/Paper"
import {useTranslation} from "react-i18next"

const StyledPaper = styled(Paper)(
    ({theme}) => `
        display: flex;
        backgroundColor: ${theme.palette.lightBackground};
        padding-top: 12px;
        padding-bottom: 12px;
        justify-content: center;
        align-items: center;
        color: ${theme.palette.customGrey.contrastText};
    `
)

const StyledLink = styled(Link)`
    text-decoration: underline;
    font-weight: normal;
    &:hover {
        text-decoration: none;
    }
`

const Footer: React.FC<PaperProps> = (args) => {
    const {t} = useTranslation()

    return (
        <StyledPaper className="footer-class" {...args}>
            <Typography variant="subtitle2" fontStyle="italic">
                {t("poweredBy")}{" "}
                <StyledLink href="//sequentech.io/" target="_blank" variant="black">
                    Sequent Tech Inc.
                </StyledLink>
            </Typography>
        </StyledPaper>
    )
}

export default Footer
