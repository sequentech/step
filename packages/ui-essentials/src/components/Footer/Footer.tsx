// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {styled} from "@mui/material/styles"
import Link from "@mui/material/Link"
import Typography from "@mui/material/Typography"
import Paper, {PaperProps} from "@mui/material/Paper"
import {Trans, useTranslation} from "react-i18next"

const StyledPaper = styled(Paper)(
    ({theme}) => `
        display: flex;
        background-color: ${theme.palette.lightBackground};
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

const CustomLink = ({title, href}: {title?: string; href?: string}) => (
    <StyledLink className="footer-link" href={href} target="_blank" rel="noopener noreferrer">
        {title}
    </StyledLink>
)

const Footer: React.FC<PaperProps> = (args) => {
    useTranslation()
    return (
        <StyledPaper className="footer-class" {...args}>
            <Typography variant="subtitle2" fontStyle="italic">
                <Trans
                    i18nKey="footer.poweredBy"
                    components={{
                        link: <CustomLink />,
                        sequent: <CustomLink href="//sequentech.io" title="Sequent Tech Inc" />,
                    }}
                >
                    <strong>Powered</strong> by&nbsp;
                    <sequent />
                </Trans>
            </Typography>
        </StyledPaper>
    )
}

export default Footer
