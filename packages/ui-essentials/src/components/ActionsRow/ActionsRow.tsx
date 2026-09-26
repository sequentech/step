// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import Button from "@mui/material/Button"
import {styled} from "@mui/material/styles"

/**
 * The row every one of the voter's screens puts its buttons in.
 *
 * This file was `ConfirmationActions`, which named one of its callers rather than what it
 * holds — and the name was wanted for the component that actually draws the confirmation
 * screen's Print and Finish.
 */
export const ActionsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    gap: 2px;
`

export const StyledButton = styled(Button)`
    display: flex;
    padding: 5px;

    span {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        padding: 5px;
    }
`
