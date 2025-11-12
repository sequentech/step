// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {styled} from "@mui/material/styles"
import Button from "@mui/material/Button"

export const CancelButton = styled(Button)`
    background-color: ${({theme}) => theme.palette.white};
    color: ${({theme}) => theme.palette.brandColor};
    border-color: ${({theme}) => theme.palette.brandColor};
    padding: 0 4rem;

    &:hover {
        background-color: ${({theme}) => theme.palette.brandColor};
    }
`

export const NextButton = styled(Button)`
    background-color: ${({theme}) => theme.palette.brandColor};
    color: ${({theme}) => theme.palette.white};
    border-color: ${({theme}) => theme.palette.brandColor};
    padding: 0 4rem;

    &:hover {
        background-color: ${({theme}) => theme.palette.white};
        color: ${({theme}) => theme.palette.brandColor};
    }
`
