<!--
SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only

-->
# ECIES Encryption Tool

This is a command line tool to encryp/decrypt using the Elliptic Curve Integrated Encryption Scheme (ECIES) Scheme.

The basic usage is:

```
java -jar target/ECIESEncryption-1.0-SNAPSHOT.jar
Usage:
  create-keys <public-key-file> <private-key-file>
  encrypt <public-key-file> <plaintext-base64>
  decrypt <private-key-file> <encrypted-text>
```

## Implementation details.

This tool creates and uses public/private key pairs with the curve type P-256 (also known as secp256r1 or prime256v1).
The main cryptography library used for implementing ECIES is Java's Bouncy Castle.

- Key Agreement: Bouncy Castle typically uses ECDH (Elliptic Curve Diffie-Hellman) for key agreement within ECIES.
- Key Derivation Function (KDF): Bouncy Castle uses a specific KDF called KDF2, which is based on the ISO18033-2 
  standard and typically uses th hash function SHA-1  to derive keys from the shared secret.
- Symmetric Encryption: For the symmetric encryption part of ECIES, Bouncy Castle often uses AES (Advanced
  Encryption Standard) in the block cipher mode CBC (Cipher Block Chaining).
- MAC: A MAC (Message Authentication Code) is used to ensure the integrity of the encrypted data. The MAC is
  generated using a key derived alongside the encryption key. The hash algorithm used is SHA-1.

## Cyphertext output generation

The final output of the Ecies encryption tool combines the IV, ephemeral public key point, ciphertext, and MAC. Format:

[IV || Ephemeral Public Key || Ciphertext || MAC]

Note: `||` is a concat operator

- IV: Initialization Vector, usually 16 bytes for AES.
- Ephemeral Public Key: The public key generated during encryption for ECDH.
- Ciphertext: The result of encrypting the plaintext with the symmetric key derived from ECDH.
- MAC: Message Authentication Code, to ensure the integrity of the ciphertext.  

# Example


```
cd /workspaces/step/packages/ECIESEncryption
mvn clean package
java -jar target/ECIESEncryption-1.0-SNAPSHOT.jar create-keys public.pem private.pem
plaintext="test data"
echo $plaintext
plaintext_b64=$(echo -n "$plaintext" | base64)
cyphertext=$(java -jar target/ECIESEncryption-1.0-SNAPSHOT.jar encrypt public.pem "$plaintext_b64")
echo $cyphertext
decoded_b64=$(java -jar target/ECIESEncryption-1.0-SNAPSHOT.jar decrypt private.pem $cyphertext)
echo -n "$decoded_b64" | base64 --decode
```

signature=$(java -jar target/ECIESEncryption-1.0-SNAPSHOT.jar sign private.pem private.pem)
java -jar target/ECIESEncryption-1.0-SNAPSHOT.jar verify public.pem private.pem MEYCIQCHZZhi2tklzQt+4fvRcdbmsLigvbSKOMjDeSfm672ucQIhAOtdNK7QtfLCfbr5of6VAluq5/Fk1WUUpQfaX/xLV662

## Development

The java code is rebuilt using:

```bash
cd /workspaces/step/packages/ECIESEncryption
mvn clean package
```
This generates a new jar file in the path
`/app/ECIESEncryption/target/ECIESEncryption-1.0-SNAPSHOT.jar` to
be used. We run windmill in a docker launched by docker compose. This deployment
expects this jar file in the path `/usr/local/bin/ecies-tool.jar`.

You can either:

a. You can attach a shell to the windmill container and directly copy the jar
inside it, which allows for a faster process, without a container rebuild:

```bash
docker compose exec windmill /bin/bash -c "cp /app/ECIESEncryption/target/ECIESEncryption-1.0-SNAPSHOT.jar /usr/local/bin/ecies-tool.jar"
```

b. Alternatively, we can copy the jar file to
`/workspaces/step/packages/windmill/external-bin/ecies-tool.jar` and then
rebuild the `sequentech.local/cargo-packages` image, which will use it within
the build:

```bash
cp /app/ECIESEncryption/target/ECIESEncryption-1.0-SNAPSHOT.jar \
  /workspaces/step/packages/windmill/external-bin/ecies-tool.jar && \
docker compose build harvest && \
docker compose stop windmill && \
docker compose up -d --no-deps windmill
```

Please ensure you update the
`/workspaces/step/packages/windmill/external-bin/ecies-tool.jar` file with the
latest version whenever you change the `ECIESEncryption` project, since
this is the file used during the generation of the windmill dockerfile in
`/workspaces/step/packages/windmill/Dockerfile.prod`.
