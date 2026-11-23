<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# ECIESEncryption threat model

ECIESEncryption is a Java command-line tool (`src/main/java/com/example/ECIESEncryptionTool.java`, one class) built into a shaded jar with a bundled cryptographic provider. The committed build, `../windmill/external-bin/ecies-tool.jar`, is installed at `/usr/local/bin/ecies-tool.jar` in the windmill and harvest images and runs on the JDK installed there. sequent-core and windmill start it as a child process from windmill workers, from velvet running inside windmill, and from harvest, which runs the `/miru/upload-signature` flow in-process. It generates the ACM key pair, signs Miru transmission files and ballot-image pages with it, wraps the transmission-package password for the receiving CCS server, and signs election returns with the PKCS#12 keys that SBEI users upload. No person runs it directly in production. It matters to an election because it handles the keys and passwords that make transmitted results authentic and confidential. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **ACM private key** (P-256, made by `create-keys`, stored by windmill in the vault): confidentiality and integrity. It signs transmission EML files, logs and ballot-image pages.
- **SBEI PKCS#12 bundles and their passwords**, uploaded through `/miru/upload-signature`: confidentiality. They sign election returns in an SBEI's name.
- **Transmission-package password** (random 64-character hex string that windmill uses to AES-encrypt the package): confidentiality. `encrypt` wraps it for the CCS public key.
- **Signatures and public keys the tool outputs**: integrity. They tie transmitted results to the server and to each SBEI.
- **CCS public key** from the election event configuration: integrity. Whoever holds the matching private key can read the package.
- **The shipped jar and its provider**: integrity of all of the above.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `create-keys` (`src/main/java/com/example/ECIESEncryptionTool.java` `createKeys`) | sequent-core `generate_ecies_key_pair`, called from windmill `get_acm_key_pair` | internal service | Argument count; `SecureRandom`; fixed curve `secp256r1`; prints only the file paths |
| `encrypt` (`encryptText`) | sequent-core `ecies_encrypt_string`, called from windmill `transmission_package.rs` `generate_encrypted_compressed_xml` | internal service; the public key comes from configuration set by admins | Argument count; key parsed as an X.509 EC public key (`loadPublicKeyFromPEM`) |
| `sign` (`signText`) | sequent-core `ecies_sign_data`, called from windmill `transmission_package.rs` (EML and logs) | internal service | Argument count; key and data read from file paths |
| `sign-bulk` (`signBulk`) | sequent-core `ecies_sign_data_bulk`, called from velvet `mcballot_images.rs` | internal service | Argument count; folder must be a directory; skips subdirectories and `.sign` files |
| `sign-ec`, `sign-rsa`, `public-key` | windmill's SBEI signature upload, behind harvest `/miru/upload-signature` (`MIRU_SIGN`) | authenticated admin (SBEI user) who supplies the PKCS#12 and its password | `KeyStore.load` checks the PKCS#12 integrity against the password |
| `decrypt`, `verify`, `verify-ec`, `verify-rsa` (`decryptText`, `verifyText`, `fullVerifyText`) | Developers and operators; no platform code calls them | operator | Argument count |
| `pom.xml` and `../windmill/external-bin/ecies-tool.jar` | Developers who commit the jar; the windmill and harvest image builds | operator | Code review |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| ECIESEncryption-T1 | Information disclosure | The transmission-package password (`encrypt`) and the PKCS#12 password (`sign-ec`, `sign-rsa`, `public-key`) are passed as command-line arguments, which other processes in the container can read | On release/10.0 only keys and data go by file path (`src/main/java/com/example/ECIESEncryptionTool.java` `signText`, `signBulk`, `signTextP12`); secrets are plain arguments. sequentech/step#3522 adds `env:NAME` arguments (`resolveSecretArgument`) and callers pass secrets through the child's environment | Open on release/10.0 (fix in sequentech/step#3522) |
| ECIESEncryption-T2 | Elevation of privilege, Information disclosure | Callers start the tool through `sh -c` with the user-supplied PKCS#12 password in the command string, so shell metacharacters in it run as commands in harvest or windmill; callers also log command lines and the ACM private key PEM (`../sequent-core/src/signatures/ecies_encrypt.rs`, `../sequent-core/src/signatures/shell.rs`, `../windmill/src/services/consolidation/rsa.rs`, `signatures.rs`, `upload_signature_service.rs`) | The tool reads its own `args` and never runs a shell (`src/main/java/com/example/ECIESEncryptionTool.java` `main`) | Open on release/10.0 (fix in sequentech/step#3522) |
| ECIESEncryption-T3 | Information disclosure | A private key leaks from the files the tool reads or writes | `src/main/java/com/example/ECIESEncryptionTool.java` `createKeys` writes keys only to the given paths and prints the paths; private keys are read from files (`readFile`, `loadPrivateKeyFromPEM`), never from arguments or standard output. Callers protect these files: sequent-core uses private temp files or a private temp directory (`../sequent-core/src/util/temp_path.rs` `generate_temp_file`, `tempfile::tempdir`) | Partial |
| ECIESEncryption-T4 | Spoofing | An election return is signed in an SBEI's name by someone who is not that SBEI | The signing key comes from the PKCS#12 store that its password unlocks; harvest accepts the upload only with `MIRU_SIGN` | Not verified |
| ECIESEncryption-T5 | Information disclosure, Tampering | The wrapped transmission password is recovered or altered | `src/main/java/com/example/ECIESEncryptionTool.java` `encryptText` uses the provider's `ECIES` cipher, with a fresh ephemeral ECDH key per call and an authentication tag that the provider checks on decrypt | Partial |
| ECIESEncryption-T6 | Tampering | Attacker-chosen data is signed with the ACM key | The tool signs only what the caller names: `signText` one file, `signBulk` the regular files of one folder, which `../sequent-core/src/signatures/ecies_encrypt.rs` `ecies_sign_data_bulk` creates with `tempfile::tempdir` per call and fills with data that windmill and velvet build | Mitigated |
| ECIESEncryption-T7 | Tampering | Known flaws in the cryptographic provider or the JVM weaken key generation, signing or PKCS#12 parsing | Provider version pinned in `pom.xml` and shaded into the jar; the JDK comes from the windmill and harvest images | Partial |
| ECIESEncryption-T8 | Tampering | The jar in the images does not match the source in `src/` | Code review; the committed jar's main class has the same methods and command strings as `src/main/java/com/example/ECIESEncryptionTool.java`, and its embedded `pom.xml` matches | Partial |
| ECIESEncryption-T9 | Denial of service | A crafted PKCS#12, key or input file makes the tool hang or exhaust memory, blocking a transmission or tally task | Each call is a separate JVM; an exception ends `main` with a non-zero exit, which sequent-core turns into an error | Not verified |
| ECIESEncryption-T10 | Spoofing, Repudiation | An operator relies on a verification command and accepts a forged signature | `src/main/java/com/example/ECIESEncryptionTool.java` `fullVerifyText` checks certificate dates and, given a CA file, the issuer name and the CA signature. No platform code calls `verify`, `verify-ec` or `verify-rsa` | Partial |
| ECIESEncryption-T11 | Repudiation | A signature or encryption cannot be traced to the task and user that asked for it | The tool keeps no state and logs nothing; windmill records SBEI signatures in event and tally annotations | Accepted (stateless CLI; attribution belongs to windmill and harvest) |

## Assumptions

- Only the windmill and harvest processes run in the containers that hold the jar, and nothing else in them can read child-process arguments, environment or temp files.
- sequent-core, windmill and velvet keep the key and data files they hand to the tool private, and keep keys, passwords and command lines out of logs (sequentech/step#3522).
- windmill keeps the ACM key pair in the vault and checks SBEI uploads before it uses them; harvest allows `/miru/upload-signature` only with `MIRU_SIGN`.
- The CCS public key in the election event configuration is authentic. Hasura permissions and admin-portal decide who can change it.
- The receiving CCS server verifies the signatures and certificates it receives.
- The shipped jar matches `src/`, and its provider and the JDK in the images are kept up to date.

## Review focus

- SBEI PKCS#12 signing, end to end.
- How sequent-core, windmill and velvet handle secrets and keys after sequentech/step#3522.
- Provider and JVM versions, and how the shipped jar is built.
- ECIES parameters against the CCS receiver's specification and current guidance.
- Resource use when the tool parses uploaded files.
- The operator-only commands (`decrypt`, `verify`, `verify-ec`, `verify-rsa`).
