// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Wrapper for QR Code
import React from "react"
import {QRCodeSVG} from "qrcode.react"

export interface QRCodeProps {
    value: string
    ariaLabelledby?: string
}

const QRCode: React.FC<QRCodeProps> = ({value, ariaLabelledby}) => (
    <QRCodeSVG value={value} aria-labelledby={ariaLabelledby} />
)

export default QRCode
