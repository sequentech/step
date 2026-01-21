// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {styled} from "@mui/material/styles"
import Stack from "@mui/material/Stack"

const PageBanner = styled(Stack)`
    justify-content: space-between;
    width: 100%;
    box-sizing: border-box;
    margin-right: auto;
    margin-left: auto;
    align-items: center;
` as typeof Stack

export default PageBanner
