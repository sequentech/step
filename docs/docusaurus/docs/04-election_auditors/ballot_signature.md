---
id: ballot_signature
title: Ballot Signature
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Ballot Signature Generation

## Introduction

This document specifies the cryptographic protocol used to generate the signature and public key associated with a digital ballot. Each ballot record contains two critical fields for verification:

* **voter\_signing\_pk**: The Base64 and DER-encoded public key of an ephemeral key pair.  
* **voter\_ballot\_signature**: The Base64-encoded signature resulting from the signing operation.

This protocol ensures that each ballot's contents are verifiably authentic and have not been tampered with. The signature is generated using an **ephemeral (single-use) key pair**, which is created for the sole purpose of signing one ballot and is then discarded. The public half of this key pair is provided to allow verification.

## Payload Preparation

The data that is cryptographically signed is a composite bytestream, constructed from two distinct parts: the Core Ballot Content and its associated metadata.

### Core Ballot Content Serialization

First, the core data fields of the ballot are isolated. This "Core Ballot Content" includes:

* version  
* issue\_date  
* contests  
* config  
* ballot\_style\_hash

This collection of data is serialized into a **deterministic binary bytestream** using the **Borsh (Binary Object Representation Serializer for Hashing)** protocol. Borsh is chosen for its strict, canonical binary representation, which guarantees that identical content will always produce an identical bytestream. This output is referred to as the serialized\_content.

### Final Payload Construction

The serialized\_content is **not** signed in isolation. Instead, it is concatenated with two external metadata identifiers to form a final, unambiguous payload. These external identifiers, which must be provided to the signing system, are:

* ballot\_id  
* election\_id

The final signed payload is constructed by concatenating these three elements, with each element being prefixed by its own length. The length is encoded as a 64-bit (8-byte) little-endian integer.

The precise structure of the final payload is as follows:

1. **ballot\_id Payload**:  
   * \[8-byte little-endian length of ballot\_id\] \+ \[ballot\_id bytestream\]  
2. **election\_id Payload**:  
   * \[8-byte little-endian length of election\_id\] \+ \[election\_id bytestream\]  
3. **Core Content Payload**:  
   * \[8-byte little-endian length of serialized\_content\] \+ \[serialized\_content bytestream\]

The final bytestream submitted to the signing algorithm is the direct concatenation of these three components: (1) \+ (2) \+ (3).

## Cryptographic Signing Process

### Ephemeral Key Generation

For each ballot, a new, cryptographically secure key pair (private and public key) is generated. The security of this protocol relies on this key pair being unique to each ballot and used only once.

### Signature Creation

The ephemeral private key is used to generate a digital signature over the **entire** final payload constructed in section 2.2.

### Output Encoding and Storage

The two resulting cryptographic artifacts are encoded as strings before being stored in the ballot record:

1. **Public Key (voter\_signing\_pk)**: The ephemeral public key is serialized according to the **Distinguished Encoding Rules (DER)** standard. This binary DER-encoded object is then **Base64 encoded** to create the final string.  
2. **Signature (voter\_ballot\_signature)**: The raw binary signature is **Base64 encoded** to create the final string.

## Verification Steps

To independently verify a ballot's signature, an auditor must perform the following steps:

1. **Obtain All Data**:  
   * The ballot record (containing the core fields and the voter\_signing\_pk and voter\_ballot\_signature).  
   * The external ballot\_id and election\_id corresponding to the ballot.  
2. **Reconstruct the Payload**:  
   * Isolate the Core Ballot Content (as specified in 2.1) and serialize it using the Borsh protocol to produce the serialized\_content.  
   * Construct the final signed payload by concatenating the length-prefixed ballot\_id, election\_id, and serialized\_content (as specified in 2.2).  
3. **Decode Cryptographic Artifacts**:  
   * Base64 decode the voter\_signing\_pk string, then DER-decode the result to reconstruct the public key.  
   * Base64 decode the voter\_ballot\_signature string to reconstruct the binary signature.  
4. **Perform Verification**:  
   * Using a standard cryptographic library, verify that the reconstructed signature is valid for the reconstructed payload, using the reconstructed public key.