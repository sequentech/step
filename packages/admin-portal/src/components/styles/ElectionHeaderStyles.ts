// Election Event A

import styled from "@emotion/styled"

export const ElectionHeaderStyles = {
    Wrapper: styled.div`
        display: flex;
        flex-direction: column;
        padding: var(--2, 16px);
        align-items: left;
        justify-content: ${({dir}) => (dir === "rtl" ? "flex-end" : "flex-start")};
    `,
    Title: styled.div`
        color: rgba(0, 0, 0, 0.87);
        font-size: 24px;
        font-family: Roboto;
        font-weight: 700;
        line-height: 32.02px;
        word-wrap: break-word;
        text-align: ${({dir}) => (dir === "rtl" ? "right" : "left")};
    `,
    SubTitle: styled.div`
        color: rgba(0, 0, 0, 0.6);
        font-size: 14px;
        font-family: Roboto;
        font-weight: 400;
        line-height: 20.02px;
        letter-spacing: 0.17px;
        word-wrap: break-word;
        text-align: ${({dir}) => (dir === "rtl" ? "right" : "left")};
    `,
}
