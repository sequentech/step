---
id: trustee_procedures
title: Trustee Procedures
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

A trustee is a member of the election board who holds one share of the
election key. The votes can be decrypted only when at least the threshold of
trustees take part in the counting, so each trustee's care for their keys is
what keeps the votes secret.

A trustee works with two things:

- their **personal account** in the admin portal, with the role `trustee` and
  their trustee name in "Act as Trustee";
- the **trustee tool** (`trustee`) on their **trustee workstation**, which
  creates and holds their keys file (`trustee.toml`).

## The trustee workstation

- Use a workstation that the election organizer provides for this purpose
  only, kept in a locked place, used by you alone, with disk encryption and
  screen lock on.
- Use only the trustee tool of the accepted TOE delivery: the `trustee`
  program inside the delivery's `braid` image, run on the trustee workstation
  without a network connection, as the Deployment Guide (AGD-04) shows
  (`podman run --rm --network none --entrypoint /usr/bin/trustee …`, defined
  there as the shell command `trustee`). The commands on this page are the
  commands of that program. Never start the `braid` image with its default
  command: that command starts the trustee server of the platform. Do not use
  a trustee tool from any other source.
- The program sees only the current directory. Run every command from the
  directory on your own medium that holds your keys file, and give file names
  relative to it, as the commands on this page do.
- The trustee tool needs no network connection to generate keys or to create a
  custody proof. Keep the workstation offline, and move the public files
  (custody request and custody proof) with a removable medium that you
  control.

## Generating your keys

Generate your keys once, before the key ceremony, on the trustee workstation:

```sh
trustee generate --trustee-config trustee.toml
trustee public-keys --trustee-config trustee.toml
```

- `trustee generate` writes the keys file under the name you give, readable by
  its owner only, and prints the two public keys. Always give
  `--trustee-config`: without it, the tool prints the whole keys file,
  including the private keys, on the screen.
- `trustee public-keys` prints the two public keys of an existing keys file as
  JSON: `signing_public_key` and `share_encryption_public_key`.

Give both public keys to the administrators, in person or on a medium you hand
over yourself. They register them under your trustee name in Settings >
"TRUSTEES" ("Create Trustee"). Check the registered keys with them, character
by character, before the key ceremony.

## Protecting your keys file

Your keys file holds your private keys in plaintext: your signing key and the
key that decrypts the shares of the election key dealt to you. Whoever has it
can act as you in the ceremonies and contribute your share to decrypting the
votes.

- Keep it on your own protected medium: the trustee workstation, or a medium
  only you control.
- Never send it by email or chat.
- Never paste it into any web page other than the ceremony step that needs
  it. In the evaluated configuration no ceremony step needs it: the steps of
  the admin portal take only the public custody request and the custody proof.
- Make two backups, in separate locked places, and keep them as securely as the
  original.
- Report the loss or exposure of the keys file or of a backup to the election
  board at once (see
  [If your keys file is lost or exposed](#if-your-keys-file-is-lost-or-exposed)).

### Handing over the keys file for your trustee server instance

Your instance of the trustee server (`braid`), the part of the TOE that takes part in the
ceremonies for you on the protected server, needs your keys file. Hand
it over only in person, to the operator who installs it for your instance, and
watch the installation. The operators keep it only in the cluster's encrypted
secrets, as the Deployment Guide describes. No copy may stay on the operator's
devices. Your instance also keeps its protocol state on the protected
server; the operators protect and back it up as the Deployment Guide
describes.

:::warning
A copy of your keys file anywhere else (a mailbox, a chat, a shared folder, a
web form, an unencrypted backup) can be used by someone else to take part in
the counting as you. If the threshold of trustees is reached by such copies,
the secrecy of the votes is lost.
:::

## Taking part in the key ceremony

Who: each trustee the ceremony names. Phase: preparation. Permission: the role
`trustee` with "Trustee Ceremony" (`trustee-ceremony`), and "Sign: confirm
custody" (`sign-key-ceremony`) where that protected action needs signatures.

When the election board has created the key ceremony, the admin portal shows
you: "You have been invited to participate in a Key Ceremony."

1. Sign in with your personal account and open the election event's Keys tab.
   Select the ceremony's key action.
2. The page "Trustee Key Ceremony" says that you participate as a trustee and
   shows your trustee name. Check that it is yours.
3. Select "Download Custody Request". The admin portal confirms "Public
   custody request downloaded successfully." The file
   `<trustee name>-custody-request.json` (your trustee name, with spaces and
   special characters replaced by "-") is public: it holds identifiers and
   your registered public keys.
4. Select "Next". The dialog "Back Up Your Local Trustee Keys" asks you to back up
   `trustee.toml` in at least two secure locations. Tick "First backup secured"
   and "Second backup secured" only when both backups exist, then select
   "Confirm Backups and Continue".
5. Move `<trustee name>-custody-request.json` to the trustee workstation, into
   the directory that holds your keys file, and create the custody proof
   there. The admin portal shows the command with its own file names; give
   the names of your files instead:

   ```sh
   trustee attest --request <trustee name>-custody-request.json \
     --trustee-config trustee.toml --output custody-proof.json
   ```

6. Move `custody-proof.json` back. On the step "Confirm Trustee Custody",
   upload it. The admin portal checks the proof when you select the file.
   If the protected action
   "Confirm custody (key ceremony)" needs signatures, sign it with your signing
   certificate.
7. The admin portal shows "Trustee custody confirmed successfully." On the Keys
   tab your row shows "Custody Confirmed".

Keep `<trustee name>-custody-request.json`: you need it again for the counting.

If the admin portal shows "Invalid custody proof, please try again", check that
you used the request of this ceremony and your own keys file, and create the
proof again. A proof is valid only for its election event, ceremony and
trustee.

## Authorising the counting

Who: each trustee the tally needs. Phase: preparation for an initialization
report, or evaluation for counting the votes. Permission: the role
`trustee` with "Trustee Ceremony" (`trustee-ceremony`), and "Sign: authorize
tally custody" (`sign-tally-key`) where that protected action needs
signatures.

When the election board has created the tally ceremony, the admin portal shows
you: "You have been invited to participate in a Tally ceremony." Check its
purpose with the board: an initialization report checks that the ballot box
is empty before voting; an electoral-results tally counts votes after voting
has closed. Both require a new custody proof bound to their own tally.

1. Sign in with your personal account, open the election event's Tally tab and
   select the tally's action.
2. Under "Trustees process" ("Upload your public custody-proof.json file") the
   admin portal shows the command to run, with the identifier of this tally:
   "Create a custody proof locally using your saved request:". Check that the
   identifier is the one of the tally the board announced.
3. On the trustee workstation, in the directory that holds your keys file and
   the saved custody request, run the command shown, with the names of your
   files. The trustee tool never overwrites a file: if the directory already
   holds a file with the name after `--output`, for example the proof of the
   key ceremony, give a new name. It has this form:

   ```sh
   trustee attest --request <trustee name>-custody-request.json \
     --trustee-config trustee.toml --tally-session-id <tally identifier> \
     --output tally-custody-proof.json
   ```

4. Upload the new `tally-custody-proof.json`. If the protected action "Authorize
   tally custody (tally)" needs signatures, sign it with your signing
   certificate.
5. Check that the section "Trustees" ("Custody confirmation status") of the
   tally counts your confirmation.

A tally proof is bound to its tally: a proof from the key ceremony or from
another tally is not accepted.

:::warning
Authorise only a tally that the election board decided. For counting votes,
this must be in the evaluation phase, after voting has closed and the audit
records were checked. For an initialization report, this must be in the
preparation phase, after the configuration was approved and before voting
starts. If the purpose, phase or identifier does not match, do not run the
command and tell the election board.
:::

## What trustees never do

- Do not use the page "Trustee Dashboard" of the admin portal. In the evaluated
  configuration trustees use only the trustee tool on their trustee
  workstation; that page would handle your keys in the browser.
- Do not run the trustee tool on any other device.
- Do not tick the backup confirmations without real backups.
- Do not give your keys file, its backups or your signing token to another
  trustee or board member, except as described in
  [A trustee leaves](03-board-roles-and-permissions.md#a-trustee-leaves).

## If your keys file is lost or exposed

1. Tell the election board at once, with the time and what happened.
2. The election board treats it as a security-relevant event and records it.
3. If a keys file is lost, restore it from a backup, only on the trustee
   workstation: both trustees are needed to count the votes.
4. If a keys file may have been exposed, the board decides under the
   electoral regulation. Before the execution phase the safe course is a new
   key ceremony with new keys for every trustee. After the start, the board
   weighs whether enough shares may be known to others to reach the threshold.

## At the end of the election

When the electoral regulation's retention period has ended and the election
execution data has been deleted, each trustee destroys their keys file and all
its backups, in the presence of another board member, and the board records it
in its minutes. The operators remove the copy held for your trustee server
instance as the Deployment Guide describes.
