# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
.PHONY: loadtesting-image
loadtesting-image: cargo-packages
	docker build -t sequentech.local/loadtesting:latest -f Dockerfile.loadtesting ..

.PHONY: cargo-packages
cargo-packages:
	docker build -t sequentech.local/cargo-packages:latest -f Dockerfile.cargo-packages .
