// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Election Styles

import styled from "@emotion/styled"

export const ContestStyles = {
    Wrapper: styled.div`
        display: flex;
        flex-direction: column;
        padding: var(--2, 16px);
        align-items: left;
    `,
    AccordionContainer: styled.div`
        display: flex;
        flex-direction: column;
        width: 372px;
    `,
    AccordionWrapper: styled.div`
        display: grid;
        grid-template-columns: 1fr 1fr;
        justify-content: space-between;
        align-items: ${({alignment = "start"}: {alignment?: "center" | "start" | "end"}) =>
            alignment};
    `,
    Title: styled.div`
        color: rgba(0, 0, 0, 0.87);
        font-size: 24px;
        font-family: Roboto;
        font-weight: 700;
        line-height: 32.02px;
        word-wrap: break-word;
    `,
    SubTitle: styled.div`
        color: rgba(0, 0, 0, 0.6);
        font-size: 14px;
        font-family: Roboto;
        font-weight: 400;
        line-height: 20.02px;
        letter-spacing: 0.17px;
        word-wrap: break-word;
    `,
}
