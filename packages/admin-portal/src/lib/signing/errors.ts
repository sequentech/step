// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Why a certificate file didn't open. A small module of its own, so the
// dialog can tell the reasons apart without loading the crypto libraries.

import {CertificateOpenFailure} from "./types"

export enum CertificateFileErrorCode {
    /** Not a PKCS#12 file at all (PEM, truncated, random bytes). */
    UnreadableFile = "UNREADABLE_FILE",
    /** The MAC (integrity) check failed or, in a MAC-less file, decryption failed. */
    WrongPassword = "WRONG_PASSWORD",
    /** A PBE, MAC or cipher forge doesn't implement. */
    UnsupportedEncryption = "UNSUPPORTED_ENCRYPTION",
    NoPrivateKey = "NO_PRIVATE_KEY",
    NoCertificate = "NO_CERTIFICATE",
    /** The key is neither RSA nor EC P-256. */
    UnsupportedKey = "UNSUPPORTED_KEY",
    /** No certificate in the file matches its private key. */
    KeyCertificateMismatch = "KEY_CERTIFICATE_MISMATCH",
}

export class CertificateFileError extends Error {
    readonly code: CertificateFileErrorCode

    constructor(code: CertificateFileErrorCode, message: string, options?: {cause?: unknown}) {
        super(message, options)
        // The portal compiles to ES5, where an Error subclass loses its prototype: restore it
        // so `instanceof CertificateFileError` holds in the production build.
        Object.setPrototypeOf(this, CertificateFileError.prototype)
        this.name = "CertificateFileError"
        this.code = code
    }
}

/** The reason the open-failures route records (the file name and this only). */
export const openFailureReason = (code: CertificateFileErrorCode): CertificateOpenFailure => {
    switch (code) {
        case CertificateFileErrorCode.WrongPassword:
            return CertificateOpenFailure.WrongPassword
        case CertificateFileErrorCode.NoPrivateKey:
            return CertificateOpenFailure.NoKey
        default:
            return CertificateOpenFailure.Unreadable
    }
}
