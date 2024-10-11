// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

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

pub mod action;
pub mod board;
pub mod datalog;
pub mod predicate;
pub mod session;
// pub mod trustee;
pub mod trustee2;
