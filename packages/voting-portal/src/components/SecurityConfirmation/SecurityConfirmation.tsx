// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useId} from "react"
import {Box, Checkbox, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {IElection, stringToHtml, translateFromPresentation} from "@sequentech/ui-core"

const StyledCheckboxWrapper = styled(Box)`
    display: flex;
    flex-direction: row;
    cursor: pointer;
    align-items: flex-start;
    padding: 10px 0;
`

const StyledCheckbox = styled(Checkbox)`
    margin-top: 4px;
    margin-right: 9px;
    padding: 0;
`

export interface SecurityConfirmationProps {
    election: IElection
    checked: boolean
    onChange: (checked: boolean) => void
}

/**
 * The eligibility declaration a voter must accept before starting to vote, shown
 * when `security_confirmation_policy` is MANDATORY. The declaration is legally
 * significant, so the checkbox has to carry it as its accessible name rather than
 * sitting next to it as unrelated text.
 */
export const SecurityConfirmation: React.FC<SecurityConfirmationProps> = ({
    election,
    checked,
    onChange,
}) => {
    const {i18n} = useTranslation()
    // The declaration is admin-authored HTML, so it labels the checkbox by
    // reference rather than by being nested inside a <label>, which may not
    // contain arbitrary markup.
    const declarationId = useId()

    const declaration =
        translateFromPresentation(election, "security_confirmation_html", i18n.language) ??
        translateFromPresentation(election, "security_confirmation_html", "en") ??
        "-"

    return (
        <StyledCheckboxWrapper className="security-confirmation" onClick={() => onChange(!checked)}>
            <StyledCheckbox
                className="security-confirmation-checkbox"
                checked={checked}
                onChange={(event) => onChange(event.target.checked)}
                // The wrapper keeps the whole row clickable for mouse users;
                // without this the wrapper would toggle a second time and cancel
                // the checkbox's own change.
                onClick={(event) => event.stopPropagation()}
                slotProps={{input: {"aria-labelledby": declarationId}}}
            />
            <Typography
                className="security-confirmation-label"
                variant="body2"
                // Never a plain Typography: the default renders a <p>, and the
                // admin HTML routinely contains block elements.
                component="div"
                marginTop="4px"
                id={declarationId}
            >
                {stringToHtml(declaration)}
            </Typography>
        </StyledCheckboxWrapper>
    )
}

export default SecurityConfirmation
