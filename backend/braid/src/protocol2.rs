/*
+--------------------------------------------------------------------------------+
| RemoteBoard                                                                    |
|                                                                                |
+------------+--------------------------------------------------------^----------+
             |                                                        |
+------------+--------------------------------------------------------+----------+
| Trustee    |                                                        |          |
|       +----v----+    +---------------+                         +----+----+     |
|       |         |    |               |                         |         |     |
|       |Message  |    |LocalBoard     |                         |Message  |     |
|       |         |    |               |                         |         |     |
|       +---------+    +---------------+                         +---------+     |
|       |Statement|    |Configuration  |    +---------+          |Statement|     |
|       |         |    |               |    |Predicate|          |         |     |
|       |Artifact |    |               |    |         |          |Artifact |     |
|       +-+-------+    |Statements ----+--->|         |          +----^----+     |
|         |            |               |    +-----+---+               |          |
|         |  Verify    |Artifacts      |          |                   |          |
|         +----------->|               |          |                   |          |
|                      +---------------+          |                   |          |
|                              ^             +----v----+        +-----+----+     |
|                              |             |Datalog  |        |Action    |     |
|                              └-------------|         |------->|          |     |
|                                Output      |         |        |          |     |
|                                Predicates  +---------+        +----------+     |
|                                                                                |
+--------------------------------------------------------------------------------+
*/

// pub mod statement2;

pub mod action;
pub mod artifact;
pub mod board;
pub mod datalog;
pub mod message;
pub mod predicate;
#[cfg(feature = "bb-test")]
pub mod session;
pub mod statement;
pub mod trustee;

pub const MAX_TRUSTEES: usize = 12;
pub const PROTOCOL_MANAGER_INDEX: usize = 1000;

pub type Hash = [u8; 64];

pub fn hash_from_vec(bytes: &[u8]) -> anyhow::Result<Hash> {
    strand::util::to_hash_array(bytes).map_err(|e| e.into())
}
