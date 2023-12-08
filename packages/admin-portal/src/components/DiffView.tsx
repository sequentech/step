import React from "react"

import styled from "@emotion/styled"

const DiffViewStyled = {
    Header: styled.span`
        font-family: Roboto;
        font-size: 14px;
        font-weight: 500;
        line-height: 24px;
        letter-spacing: 0.4000000059604645px;
        text-align: left;
        text-transform: uppercase; 
    `,
    Content: styled.div`
        width: 100%;
        display: flex;
        flex-direction: column;
        gap: 12px;
    `,
    Container: styled.div`
        display: flex;
        justify-content: space-around;
        gap: 16px;
    `,
    Block: styled.div`
        width: 100%;
        display: flex;
        background-color: #f5f5f5;
        padding: 16px;
        height: 100%;
    `,
    Json: styled.div`
        width: 100%;
    `,
    Removed: styled.pre`
        width: 100%;
        font-size: 12px;
        background-color: #fa958e;
        text-decoration: line-through;
    `,
    Added: styled.pre`
        width: 100%;
        font-size: 12px;
        background-color: #43e3a1;
    `,
    Line: styled.pre`
        width: 100%;
        font-size: 12px;
    `,
}

export const DiffView: React.FC<{diff: {[key:string]: string}[]}> = ({ diff }) => {
    return (
      <DiffViewStyled.Container>
        <DiffViewStyled.Content>
            <DiffViewStyled.Header>
                Actual
            </DiffViewStyled.Header>
            <DiffViewStyled.Block>
                <DiffViewStyled.Json>
                    {diff.map((line: any, index: number) => (
                        !line.added ? line.removed ? (
                            <DiffViewStyled.Removed key={index}>
                                {line.value}
                            </DiffViewStyled.Removed>
                        ) : (
                            <DiffViewStyled.Line key={index}>
                                {line.value}
                            </DiffViewStyled.Line>
                        ) : null
                    ))}
                </DiffViewStyled.Json>
            </DiffViewStyled.Block>
        </DiffViewStyled.Content>

        <DiffViewStyled.Content>
            <DiffViewStyled.Header>
                CHANGES TO PUBLISH
            </DiffViewStyled.Header>
            <DiffViewStyled.Block>
                <DiffViewStyled.Json>
                    {diff.map((line: any, index: number) => (
                        !line.removed ? line.added ? (
                            <DiffViewStyled.Added key={index}>
                                {line.value}
                            </DiffViewStyled.Added>
                        ) : (
                            <DiffViewStyled.Line key={index}>
                                {line.value}
                            </DiffViewStyled.Line>
                        ) : null
                    ))}
                </DiffViewStyled.Json>
            </DiffViewStyled.Block>
        </DiffViewStyled.Content>
      </DiffViewStyled.Container>
    );
  };
