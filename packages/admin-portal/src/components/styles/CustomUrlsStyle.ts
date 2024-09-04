// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import styled from "@emotion/styled"

export const CustomUrlsStyle = {
    InputWrapper: styled.div`
        width: 100%;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
    `,

    InputLabelWrapper: styled.div`
        display: flex;
        flex-direction: row;
        align-items: center;
        gap: 8px;
    `,
    ErrorText: styled.div`
        color: red;
        font-size: 12px;
    `,
}
