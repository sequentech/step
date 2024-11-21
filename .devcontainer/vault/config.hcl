# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

ui = true

# mlock on some development environments might not be available and
# might prevent vault from starting.
disable_mlock = true

storage "file" {
    path = "/vault"
}

listener "tcp" {
    address = "0.0.0.0:8201"
    tls_disable = true
}
