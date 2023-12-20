import React from "react"

import {Identifier} from "react-admin"

type TCommunicationTemplateEdit = {
    close?: () => void
    id?: Identifier | undefined
}

export const CommunicationTemplateEdit: React.FC<TCommunicationTemplateEdit> = () => {
    return (
        <h1>Communication Template Edit</h1>
    )
}
