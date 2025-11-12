// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Election Event A

import {styled} from "@mui/material/styles"

export const ElectionHeaderStyles = {
    Wrapper: styled("div")`
        display: flex;
        flex-direction: column;
        padding: var(--2, 16px);
        align-items: left;
    `,
    ThinWrapper: styled("div")`
        display: flex;
        flex-direction: column;
        padding: var(--2, 0);
        align-items: left;
    `,
    Title: styled("div")`
        color: rgba(0, 0, 0, 0.87);
        font-size: 24px;
        font-family: Roboto;
        font-weight: 700;
        line-height: 32.02px;
        word-wrap: break-word;
    `,
    SubTitle: styled("div")`
        color: rgba(0, 0, 0, 0.6);
        font-size: 14px;
        font-family: Roboto;
        font-weight: 400;
        line-height: 20.02px;
        letter-spacing: 0.17px;
        word-wrap: break-word;
    `,
    AccordionTitle: styled("div")`
        color: rgba(0, 0, 0, 0.6);
        font-size: 16px;
        font-family: Roboto;
        font-weight: 700;
        line-height: 20.02px;
        letter-spacing: 0.17px;
        word-wrap: break-word;
    `,
}
