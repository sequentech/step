// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {styled} from "@mui/material/styles"
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

/*
 * `styled("a")`, not `styled(Link)`, and this is load-bearing.
 *
 * The shared theme sets every MUI `Link` to `LinkBehavior`, which is react-router's —
 * so this component could not render outside a `Router`, and the Election Architect's
 * ballot preview has none. It failed there with *"Cannot destructure property
 * 'basename'"*, a message that says nothing about the cause. `ReviewLayout` hit the
 * same wall when it was lifted and took the same fix.
 *
 * It is also correct on its own terms: this link is external and opens in a new tab,
 * so routing was never wanted. The portal is unaffected — an `<a href target=_blank>`
 * is what a router `Link` degrades to for an off-site URL.
 */
const StyledLink = styled("a")(({theme}) => ({
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

    if (!poweredByString.includes("<0>") && !poweredByString.includes("<1>")) {
        return (
            <StyledPaper role="contentinfo" component="footer" className="footer-class" {...args}>
                <Typography variant="subtitle2" fontStyle="italic" color="error">
                    Error: Invalid translation for footer.poweredBy. It must contain `&lt;1
                    &gt;``&lt;1 /&gt;`.
                </Typography>
            </StyledPaper>
        )
    }

    return (
        <StyledPaper role="contentinfo" component="footer" className="footer-class" {...args}>
            <Typography variant="subtitle2" fontStyle="italic">
                <Trans
                    i18nKey="footer.poweredBy"
                    components={[
                        <CustomLink />,
                        <CustomLink href="//sequentech.io" title="Sequent Tech Inc" />,
                    ]}
                />
            </Typography>
        </StyledPaper>
    )
}

export default Footer
