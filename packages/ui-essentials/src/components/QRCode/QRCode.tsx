// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Wrapper for QR Code
import React from "react"
import {QRCodeSVG} from "qrcode.react"

// React 19 compatibility wrapper for QRCodeSVG
const QRCodeSVGFixed: React.FC<any> = (props) => {
    const QR = QRCodeSVG as any;
    return <QR {...props} />;
};

export interface QRCodeProps {
    value: string
    ariaLabelledby?: string
}

const QRCode: React.FC<QRCodeProps> = ({value, ariaLabelledby}) => (
    <QRCodeSVGFixed value={value} aria-labelledby={ariaLabelledby} className="qr-code-svg" />
)

export default QRCode
