// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Verificatum's cryptographic primitives: hash functions, the PRG, and random
//! oracles (VMNV §5), plus the global prefix ρ every oracle query is salted
//! with (VMNV §9.3 step 4).
//!
//! This is the layer that decides whether the whole interop approach is viable.
//! The proof *algebra* already agrees between braid and Verificatum; what does
//! not agree is the transcript, and every challenge in every proof is derived
//! through the constructions below. If these reproduce VMN's values bit for bit
//! then a Verificatum-compatible emitter in braid is a matter of plumbing; if
//! they cannot, the approach stops here.
//!
//! Each construction is small, and each is pinned by a golden value taken from a
//! real VMN run via `vmnv -t`.

use sha2::Digest;

use crate::wire::bytetree::ByteTree;

/// Which SHA-2 variant VMN is configured with (VMNV §5.1). VMN 3.1.0 supports
/// only these three — notably **not** SHA-3, which is what braid uses natively,
/// and one of the reasons braid's existing transcripts are incompatible.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Hashfunction {
    Sha256,
    Sha384,
    Sha512,
}

impl Hashfunction {
    /// Parse the string form used in protocol info files (`rohash`, `prg`).
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "SHA-256" => Some(Hashfunction::Sha256),
            "SHA-384" => Some(Hashfunction::Sha384),
            "SHA-512" => Some(Hashfunction::Sha512),
            _ => None,
        }
    }

    /// The name as it appears in a protocol info file.
    pub fn name(&self) -> &'static str {
        match self {
            Hashfunction::Sha256 => "SHA-256",
            Hashfunction::Sha384 => "SHA-384",
            Hashfunction::Sha512 => "SHA-512",
        }
    }

    /// `outlen(H)` in bits (VMNV §5.1).
    pub fn outlen_bits(&self) -> usize {
        match self {
            Hashfunction::Sha256 => 256,
            Hashfunction::Sha384 => 384,
            Hashfunction::Sha512 => 512,
        }
    }

    /// `outlen(H)` in bytes.
    pub fn outlen(&self) -> usize {
        self.outlen_bits() / 8
    }

    /// `H(data)`.
    pub fn hash(&self, data: &[u8]) -> Vec<u8> {
        match self {
            Hashfunction::Sha256 => sha2::Sha256::digest(data).to_vec(),
            Hashfunction::Sha384 => sha2::Sha384::digest(data).to_vec(),
            Hashfunction::Sha512 => sha2::Sha512::digest(data).to_vec(),
        }
    }
}

/// VMN's pseudo-random generator (VMNV §5.2).
///
/// From a seed `s` of `outlen(H)` bits it emits `r_0 | r_1 | r_2 | ...` where
///
/// ```text
/// r_i = H(s || bytes_4(i))
/// ```
///
/// i.e. each block re-hashes the seed with a 4-byte big-endian counter. Note the
/// counter starts at 0 and the seed is *not* itself part of the output.
pub struct Prg {
    hash: Hashfunction,
    seed: Vec<u8>,
}

impl Prg {
    /// Seed the PRG. `seedlen(PRG) = outlen(H)`.
    pub fn new(hash: Hashfunction, seed: &[u8]) -> Self {
        Prg {
            hash,
            seed: seed.to_vec(),
        }
    }

    /// The number of seed bytes this PRG requires.
    pub fn seed_len(hash: Hashfunction) -> usize {
        hash.outlen()
    }

    /// Generate `len` pseudo-random bytes.
    pub fn generate(&self, len: usize) -> Vec<u8> {
        let block = self.hash.outlen();
        let mut out = Vec::with_capacity(len.next_multiple_of(block.max(1)));
        let mut counter: u32 = 0;
        while out.len() < len {
            let mut input = Vec::with_capacity(self.seed.len() + 4);
            input.extend_from_slice(&self.seed);
            input.extend_from_slice(&counter.to_be_bytes());
            out.extend_from_slice(&self.hash.hash(&input));
            counter += 1;
        }
        out.truncate(len);
        out
    }
}

/// VMN's flexible random oracle (VMNV §5.3).
///
/// `RO(d)` for an output length of `n_out` bits:
///
/// 1. `s = H(bytes_4(n_out) || d)` — the output length is prefixed, which is
///    what separates oracles of different widths built on the same `H`;
/// 2. take the first `ceil(n_out / 8)` bytes of `PRG(s)`;
/// 3. if `n_out mod 8 != 0`, zero the leading `8 - (n_out mod 8)` bits, so the
///    result reads directly as a non-negative integer of nominal bit length
///    `n_out`.
pub struct RandomOracle {
    hash: Hashfunction,
    out_bits: usize,
}

impl RandomOracle {
    pub fn new(hash: Hashfunction, out_bits: usize) -> Self {
        RandomOracle { hash, out_bits }
    }

    /// Evaluate the oracle on `data`.
    pub fn eval(&self, data: &[u8]) -> Vec<u8> {
        let mut input = Vec::with_capacity(4 + data.len());
        input.extend_from_slice(&(self.out_bits as u32).to_be_bytes());
        input.extend_from_slice(data);
        let seed = self.hash.hash(&input);

        let out_bytes = self.out_bits.div_ceil(8);
        let mut out = Prg::new(self.hash, &seed).generate(out_bytes);

        let excess = self.out_bits % 8;
        if excess != 0 && !out.is_empty() {
            // Zero the top (8 - excess) bits of the first byte.
            out[0] &= 0xFFu8 >> (8 - excess);
        }
        out
    }
}

/// The parameters that go into the global prefix ρ, as read from a protocol
/// info file (VMNV §7.2) and the proof directory.
#[derive(Clone, Debug)]
pub struct PrefixParams {
    /// VMN version string, e.g. `"3.1.0"`.
    pub version: String,
    /// Session identifier from the protocol info file.
    pub sid: String,
    /// Auxiliary session identifier from the proof directory, e.g. `"default"`.
    pub auxsid: String,
    /// `n_r`, the statistical distance parameter (`statdist`).
    pub n_r: u32,
    /// `n_v`, challenge bit length (`vbitlenro`).
    pub n_v: u32,
    /// `n_e`, batching-component bit length (`ebitlenro`).
    pub n_e: u32,
    /// The PRG hash name, e.g. `"SHA-256"` (`prg`).
    pub prg: String,
    /// The **full** marshalled group string from `<pgroup>`, comment included.
    pub pgroup: String,
    /// The random-oracle hash name (`rohash`).
    pub rohash: String,
}

/// Compute the global prefix ρ fed into every random oracle query
/// (VMNV §9.3 step 4).
///
/// ```text
/// rho = H( node( leaf(version),
///                leaf(sid || "." || auxsid),
///                leaf(bytes_4(n_r)),
///                leaf(bytes_4(n_v)),
///                leaf(bytes_4(n_e)),
///                leaf(prg),
///                leaf(pgroup),
///                leaf(rohash) ) )
/// ```
///
/// Two details that are easy to get wrong and that were taken from VMN's own
/// source rather than the prose: the session identifier is the **concatenation**
/// `sid.auxsid`, and `pgroup` is the entire `<pgroup>` string *including* the
/// `ECqPGroup(P-256)::` comment prefix, not just the hex payload.
pub fn global_prefix(hash: Hashfunction, params: &PrefixParams) -> Vec<u8> {
    let rosid = format!("{}.{}", params.sid, params.auxsid);
    let int_leaf = |n: u32| ByteTree::leaf(n.to_be_bytes().to_vec());

    let tree = ByteTree::node(vec![
        ByteTree::leaf(params.version.as_bytes().to_vec()),
        ByteTree::leaf(rosid.into_bytes()),
        int_leaf(params.n_r),
        int_leaf(params.n_v),
        int_leaf(params.n_e),
        ByteTree::leaf(params.prg.as_bytes().to_vec()),
        ByteTree::leaf(params.pgroup.as_bytes().to_vec()),
        ByteTree::leaf(params.rohash.as_bytes().to_vec()),
    ]);

    hash.hash(&tree.to_bytes())
}

/// `RO_seed` — the oracle producing PRG seeds, with output length
/// `seedlen(PRG)` (VMNV §8.1).
pub fn ro_seed(hash: Hashfunction) -> RandomOracle {
    RandomOracle::new(hash, hash.outlen_bits())
}

/// `RO_challenge` — the oracle producing challenges, with output length `n_v`
/// (VMNV §8.1).
pub fn ro_challenge(hash: Hashfunction, n_v: usize) -> RandomOracle {
    RandomOracle::new(hash, n_v)
}

/// Evaluate a random oracle on `ρ ‖ ser(data)` — the shape every Fiat–Shamir
/// query in VMN takes.
///
/// Matches `ChallengerRO.challenge`, which feeds the global prefix into the
/// digest and then the byte tree: the prefix is **raw bytes**, not a byte tree
/// node, so it is concatenated ahead of the serialized tree rather than
/// wrapped with it.
pub fn oracle_query(
    hash: Hashfunction,
    out_bits: usize,
    rho: &[u8],
    data: &ByteTree,
) -> Vec<u8> {
    let tree_bytes = data.to_bytes();
    let mut input = Vec::with_capacity(rho.len() + tree_bytes.len());
    input.extend_from_slice(rho);
    input.extend_from_slice(&tree_bytes);
    RandomOracle::new(hash, out_bits).eval(&input)
}

/// Widen a full public key `pk = (g, y)` to ciphertext width `ω`
/// (`ProtocolElGamal.getWidePublicKey`).
///
/// **This is not what VMNV §8.3's "pk ∈ C_κ" suggests.** The query that derives
/// a shuffle proof's batching seed does not use the stored `FullPublicKey.bt`
/// when ω > 1; it uses the key widened component-wise:
///
/// ```text
/// width 1: (g, y)                              -- unchanged
/// width w: ( (g, ..., g)_w , (y, ..., y)_w )
/// ```
///
/// Using the un-widened key yields a wrong seed and therefore a wrong challenge,
/// with no diagnostic beyond a failed verification.
pub fn wide_public_key(pk: &ByteTree, width: usize) -> crate::wire::error::Result<ByteTree> {
    if width == 1 {
        return Ok(pk.clone());
    }
    let parts = pk.as_node_of(2)?;
    let repeat = |e: &ByteTree| ByteTree::node(vec![e.clone(); width]);
    Ok(ByteTree::node(vec![repeat(&parts[0]), repeat(&parts[1])]))
}

/// The batching seed `s` of a proof of a shuffle (VMNV §8.3):
///
/// ```text
/// s = RO_seed(rho | node(g, h, u, pk, w, w'))
/// ```
///
/// Output length is `8 * seedlen(PRG)` bits, i.e. one PRG seed.
pub fn pos_seed(
    hash: Hashfunction,
    rho: &[u8],
    g: &ByteTree,
    h: &ByteTree,
    u: &ByteTree,
    pk: &ByteTree,
    w: &ByteTree,
    w_prime: &ByteTree,
) -> Vec<u8> {
    let data = ByteTree::node(vec![
        g.clone(),
        h.clone(),
        u.clone(),
        pk.clone(),
        w.clone(),
        w_prime.clone(),
    ]);
    oracle_query(hash, hash.outlen_bits(), rho, &data)
}

/// The batching seed `s` of a proof of correct decryption (VMNV §8.6):
///
/// ```text
/// s = RO_seed(rho | node( node(g, w), node(Gamma, node(f_1, ..., f_k)) ))
/// ```
///
/// Two asymmetries with [`pos_seed`] that are easy to get wrong:
///
/// - `g` is the **plain** group generator, not widened to omega, even though `w`
///   is a width-omega ciphertext array. [`pos_seed`] widens *its* public key
///   ([`wide_public_key`]), so neither "always widen" nor "never widen" is a
///   safe rule — each query has to be checked against the implementation.
/// - `w` is the list being decrypted, i.e. the *final* shuffled output
///   `L_lambda_a`, not the original input ciphertexts.
///
/// # `factors` covers every party
///
/// One decryption-factor array per party, in party order, for **all `k`
/// parties** — not only those named by `CorrectIndices.bt`. That set (`Δ`,
/// of size `λ`) selects which factors are Lagrange-combined *later*; the seed
/// commits to all of them regardless.
///
/// **This is not covered by the reference corpus**, which has `k = 1` and so
/// cannot distinguish "all parties" from "only the correct ones" — both produce
/// the same bytes. It is taken from the verifier's source
/// (`getDecryptionFactorsBT` loops to `v.k`), and only becomes testable with a
/// multi-party decryption corpus.
pub fn dec_seed(
    hash: Hashfunction,
    rho: &[u8],
    g: &ByteTree,
    ciphertexts: &ByteTree,
    gamma: &ByteTree,
    factors: &[ByteTree],
) -> Vec<u8> {
    let bt_in = ByteTree::node(vec![g.clone(), ciphertexts.clone()]);
    let bt_out = ByteTree::node(vec![gamma.clone(), ByteTree::node(factors.to_vec())]);
    let data = ByteTree::node(vec![bt_in, bt_out]);
    oracle_query(hash, hash.outlen_bits(), rho, &data)
}

/// The challenge `v` of a proof of correct decryption (VMNV §8.6):
///
/// ```text
/// v = RO_challenge(rho | node(leaf(s), node(tau_1^dec, ..., tau_k^dec)))
/// ```
///
/// `commitments` holds one commitment per party, in party order.
pub fn dec_challenge(
    hash: Hashfunction,
    n_v: usize,
    rho: &[u8],
    seed: &[u8],
    commitments: &[ByteTree],
) -> Vec<u8> {
    let data = ByteTree::node(vec![
        ByteTree::leaf(seed.to_vec()),
        ByteTree::node(commitments.to_vec()),
    ]);
    oracle_query(hash, n_v, rho, &data)
}

/// The challenge `v` of a proof of a shuffle (VMNV §8.3):
///
/// ```text
/// v = RO_challenge(rho | node(leaf(s), tau_pos))
/// ```
///
/// `tau_pos` is the proof commitment exactly as stored in
/// `PoSCommitment<l>.bt`.
pub fn pos_challenge(
    hash: Hashfunction,
    n_v: usize,
    rho: &[u8],
    seed: &[u8],
    tau_pos: &ByteTree,
) -> Vec<u8> {
    let data = ByteTree::node(vec![ByteTree::leaf(seed.to_vec()), tau_pos.clone()]);
    oracle_query(hash, n_v, rho, &data)
}
