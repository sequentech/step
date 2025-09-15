# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
.PHONY: loadtesting-image
loadtesting-image: cargo-packages
	docker build -t sequentech.local/loadtesting:v9.1.0-rc.4 -f Dockerfile.loadtesting ..

.PHONY: cargo-packages
cargo-packages:
	docker build -t sequentech.local/cargo-packages:v9.1.0-rc.4 -f Dockerfile.cargo-packages .
