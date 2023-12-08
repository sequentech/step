import React from "react"

import styled from "@emotion/styled"

import { diffLines } from 'diff';
import { CircularProgress } from "@mui/material"

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
    Loading: styled.div`
        display: flex;
        height: 60vh;
        justify-content: center;
        align-items: center;
    `
}

type TDiffView<T> = {
    type?: 'simple' | 'modify';
    diffTitle: string;
    currentTitle: string;
    current: T;
    modify: T;
};

const DiffViewMemo = React.memo(<T extends {}>({ current, currentTitle, modify, diffTitle, type = 'modify'}: TDiffView<T>) => {
    const [diff, setDiff] = React.useState<any>('')
    const [oldJsonString, setOldJsonString] = React.useState<string>('')
    const [newJsonString, setNewJsonString] = React.useState<string>('')

    React.useEffect(() => {
        setNewJsonString(JSON.stringify(modify, null, 2))
        setOldJsonString(JSON.stringify(current, null, 2))
    }, [])

    React.useEffect(() => {
        if (oldJsonString && newJsonString) {
            const diffText: any = diffLines(oldJsonString, newJsonString)
    
            console.log(diffText);
    
            setDiff(diffText)
        }
    }, [oldJsonString, newJsonString])

    if (!diff) {
        return (
            <DiffViewStyled.Loading>
                <CircularProgress />
            </DiffViewStyled.Loading>
        )
    }

    return (
      <DiffViewStyled.Container>
        <DiffViewStyled.Content>
            <DiffViewStyled.Header>
                {currentTitle}
            </DiffViewStyled.Header>
            <DiffViewStyled.Block>
                <DiffViewStyled.Json>
                    {diff.map((line: any, index: number) => (
                        !line.added ? line.removed && type === 'modify' ? (
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

        {type === 'modify' && (
            <DiffViewStyled.Content>
                <DiffViewStyled.Header>
                    {diffTitle}
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
        )}
      </DiffViewStyled.Container>
    );
});

DiffViewMemo.displayName = 'DiffView';

export const DiffView = DiffViewMemo;


