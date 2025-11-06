// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
) as typeof Paper

const StyledLink = styled(Link)(({theme}) => ({
    "textDecoration": "underline",
    "fontWeight": "normal",
    "color": theme.palette.blue.dark,
    "&:hover": {
        textDecoration: "none",
    },
}))

const CustomLink = ({title, href}: {title?: string; href?: string}) => (
    <StyledLink className="footer-link" href={href} target="_blank" rel="noopener noreferrer">
        {title}
    </StyledLink>
)

const Footer: React.FC<PaperProps> = (args) => {
    const {t} = useTranslation()
    const poweredByString = t("footer.poweredBy")

    if (!poweredByString.includes("<sequent />")) {
        return (
            <StyledPaper role="contentinfo" component="footer" className="footer-class" {...args}>
                <Typography variant="subtitle2" fontStyle="italic" color="error">
                    Error: Invalid translation for footer.poweredBy. It must contain `&lt;sequent
                    /&gt;`.
                </Typography>
            </StyledPaper>
        )
    }

    return (
        <StyledPaper role="contentinfo" component="footer" className="footer-class" {...args}>
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

declare global {
    // eslint-disable-next-line @typescript-eslint/no-namespace
    namespace JSX {
        interface IntrinsicElements {
            sequent: React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>
        }
    }
}

export default Footer
