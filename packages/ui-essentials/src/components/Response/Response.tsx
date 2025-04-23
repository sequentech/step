// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import Image from "mui-image"
import {ECandidatesIconCheckboxPolicy} from "@sequentech/ui-core"
import Candidate from "../Candidate/Candidate"

export interface IResponseProps {
    // UI-related props only
    title: string | React.ReactNode
    description: string | React.ReactNode
    isActive: boolean
    checked: boolean
    setChecked: (value: boolean) => void
    url?: string
    hasCategory?: boolean
    isWriteIn: boolean
    writeInValue?: string
    setWriteInText: (text: string) => void
    isInvalidVote?: boolean
    isInvalidWriteIn?: boolean
    shouldDisable: boolean
    iconCheckboxPolicy?: ECandidatesIconCheckboxPolicy
    imageUrl?: string
    imageSrc?: string
}

const Response: React.FC<IResponseProps> = ({
    title,
    description,
    isActive,
    checked,
    setChecked,
    url,
    hasCategory,
    isWriteIn,
    writeInValue,
    setWriteInText,
    isInvalidVote,
    isInvalidWriteIn,
    shouldDisable,
    iconCheckboxPolicy,
    imageUrl,
    imageSrc,
}) => {
    return (
        <Candidate
            title={title}
            description={description}
            isActive={isActive}
            checked={checked}
            setChecked={setChecked}
            url={url}
            hasCategory={hasCategory}
            isWriteIn={isWriteIn}
            writeInValue={writeInValue}
            setWriteInText={setWriteInText}
            isInvalidVote={isInvalidVote}
            isInvalidWriteIn={isInvalidWriteIn}
            shouldDisable={shouldDisable}
            iconCheckboxPolicy={iconCheckboxPolicy}
        >
            {imageUrl ? <Image src={imageSrc ?? ""} duration={100} /> : null}
        </Candidate>
    )
}

export default Response
