---
id: admin_portal_tutorials_tally-ceremony
title: Tally Ceremony
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

For a manual tally ceremony, participating trustees authorize the current tally
from the administration portal before the administrator starts tallying.

1. Open the election event's tally ceremony as a participating trustee.
2. Upload the public ceremony file saved during the key ceremony. Use the same
   `<trustee>-custody-request.json` file; you do not need to download another one.
3. Wait for the running server trustee to create a fresh proof for this tally.
4. Complete the signing dialog if an additional approval signature is required.
5. Continue once custody is confirmed. The administrator can start tallying when
   the required number of trustees have authorized it.

If the server has not responded, check that its runner is online and upload the
file again. No command on the trustee server is needed. Uploading a file from
another trustee or key ceremony is rejected, and a signed proof from a previous
tally cannot authorize the current one.

Offline and browser trustees can still create and upload a signed custody proof
with the current tally session UUID. See [trustee configuration](../../07-developers/09-wbraid/trustee-configuration.md#manual-custody-proofs).

The saved public ceremony file supports this manual approval procedure. Recovering
a trustee after losing its server requires separate backups of its private keys
and protocol state; uploading the public file does not restore either.
