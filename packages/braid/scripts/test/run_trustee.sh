# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
# you must set this value
STEP_HOME=.
/usr/bin/time -f "%E real\t%M kb\t%P cpu\t%U user\t%S sys" $STEP_HOME/packages/target/release/main_m --b3-url http://127.0.0.1:50051 --trustee-config trustee.toml --no-cache
