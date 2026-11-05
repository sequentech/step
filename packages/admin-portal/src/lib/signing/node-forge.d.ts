// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// certificate.ts imports only forge's PKCS#12 modules (the index also pulls in
// TLS, SSH and PKCS#7). Each one registers itself on the shared forge object.
declare module "node-forge/lib/forge" {
    import forge from "node-forge"
    export default forge
}
