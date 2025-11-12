---
id: third_party_deps
title: Third-Party Dependencies
---

# Third-Party Dependencies Reference

This document provides a comprehensive listing of all third-party dependencies used across the Sequent Voting Platform (SVP) packages, including their licenses and descriptions.

## Overview

The SVP monorepo contains packages written in multiple languages and using different package managers:

- **Rust packages**: Managed with Cargo, using dependencies from [crates.io](https://crates.io)
- **TypeScript/JavaScript packages**: Managed with npm/yarn, using dependencies from [npmjs.com](https://npmjs.com)
- **Java packages**: Managed with Maven, using dependencies from [Maven Central](https://central.sonatype.com)

Each package's dependencies are listed below with their version, license, and description information.

## License Compliance

This documentation lists the licenses of all third-party dependencies for compliance and legal review purposes. Please ensure that all license requirements are met when distributing or deploying the Sequent Voting Platform.

## ECIESEncryption

ECIESEncryption is a Java-based package providing elliptic curve encryption capabilities.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| com.madgag.spongycastle:core | 1.58.0.0 | Bouncy Castle Licence | Spongy Castle is a package-rename (org.bouncycastle.* to org.spongycastle.*) of Bouncy Castle intended for the Android platform. Android unfortunately ships with a stripped-down version of Bouncy Castle, which prevents easy upgrades - Spongy Castle overcomes this and provides a full, up-to-date version of the Bouncy Castle cryptographic libs. |
| com.madgag.spongycastle:prov | 1.58.0.0 | Bouncy Castle Licence | Spongy Castle is a package-rename (org.bouncycastle.* to org.spongycastle.*) of Bouncy Castle intended for the Android platform. Android unfortunately ships with a stripped-down version of Bouncy Castle, which prevents easy upgrades - Spongy Castle overcomes this and provides a full, up-to-date version of the Bouncy Castle cryptographic libs. |

## Admin Portal

The admin portal is a React-based web application for administrative functions.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| @apollo/client | 3.8.0 | MIT | A fully-featured caching GraphQL client. |
| @bb-tech/ra-components | 2.0.6 | MIT | Additional opensource components for react-admin 4.x |
| @emotion/react | 11.10.0 | MIT | > Simple styling in React. |
| @emotion/styled | 11.10.0 | MIT | styled API for emotion |
| @fortawesome/fontawesome-svg-core | ^6.1.1 | MIT | The iconic font, CSS, and SVG framework |
| @fortawesome/free-brands-svg-icons | ^6.1.1 | (CC-BY-4.0 AND MIT) | The iconic font, CSS, and SVG framework |
| @fortawesome/free-solid-svg-icons | ^6.1.1 | (CC-BY-4.0 AND MIT) | The iconic font, CSS, and SVG framework |
| @fortawesome/react-fontawesome | ^0.1.18 | MIT | Official React component for Font Awesome |
| @mui/material | 5.13.3 | MIT | Material UI is an open-source React component library that implements Google's Material Design. It's comprehensive and can be used in production out of the box. |
| @mui/styles | ^5.14.19 | MIT | MUI Styles - The legacy JSS-based styling solution of Material UI. |
| @mui/utils | ^5.14.19 | MIT | Utility functions for React components. |
| @mui/x-data-grid | 6.18.2 | MIT | The Community plan edition of the MUI X Data Grid components. |
| @react-page/react-admin | 5.4.4 | MIT | see the ReactAdmin example in the docs |
| @reduxjs/toolkit | 1.9.5 | MIT | The official, opinionated, batteries-included toolset for efficient Redux development |
| @tinymce/tinymce-react | ^4.3.2 | MIT | Official TinyMCE React Component |
| @types/diff | ^5.0.8 | MIT | Stub TypeScript definitions entry for diff, which provides its own types definitions |
| antd | ^5.21.5 | MIT | An enterprise-class UI design language and React components implementation |
| apexcharts | 3.41.1 | SEE LICENSE IN LICENSE | A JavaScript Chart Library |
| buffer | ^6.0.3 | MIT | Node.js Buffer API, for the browser |
| diff | ^5.1.0 | BSD-3-Clause | A JavaScript text diff implementation. |
| dompurify | ^3.2.4 | (MPL-2.0 OR Apache-2.0) | DOMPurify is a DOM-only, super-fast, uber-tolerant XSS sanitizer for HTML, MathML and SVG. It's written in JavaScript and works in all modern browsers (Safari, Opera (15+), Internet Explorer (10+), Firefox and Chrome - as well as almost anything else usin |
| fs-extra | ^11.2.0 | MIT | fs-extra contains methods that aren't included in the vanilla Node.js fs package. Such as recursive mkdir, copy, and remove. |
| graphql | ^16.8.1 | MIT | A Query Language and Runtime which can target any service. |
| i18next | ^21.8.16 | MIT | i18next internationalization framework |
| i18next-browser-languagedetector | ^6.1.4 | MIT | language detector used in browser environment for i18next |
| intl-tel-input | ^24.5.0 | MIT | A JavaScript plugin for entering and validating international telephone numbers |
| jotai | ^2.6.0 | MIT | 👻 Primitive and flexible state management for React |
| json-edit-react | ^1.17.1 | MIT | React component for editing or viewing JSON/object data |
| keycloak-js | ^22.0.1 | Apache-2.0 | A client-side JavaScript OpenID Connect library that can be used to secure web applications. |
| lodash | ^4.17.21 | MIT | Lodash modular utilities. |
| moment-timezone | ^0.5.46 | MIT | Parse and display moments in any timezone. |
| mui-image | 1.0.7 | ISC | Display images as per the Material guidelines. For React apps using Material-UI. |
| process | ^0.11.10 | MIT | process information for node.js and browsers |
| ra-data-hasura | 0.6.0 | MIT | A data provider for connecting react-admin to a Hasura endpoint |
| react | 18.1.0 | MIT | React is a JavaScript library for building user interfaces. |
| react-admin | 4.12.4 | MIT | A frontend Framework for building admin applications on top of REST services, using ES6, React and Material UI |
| react-admin-import-csv | 4.0.1 | MIT | CSV import button for react-admin |
| react-admin-json-view | 2.0.0 | MIT | JSON field and input for react-admin. |
| react-apexcharts | 1.4.1 | SEE LICENSE IN LICENSE | React.js wrapper for ApexCharts |
| react-diff-view | ^3.2.0 | MIT | A git diff component to consume the git unified diff output. |
| react-dnd | 16.0.1 | MIT | Drag and Drop for React |
| react-dnd-html5-backend | 16.0.1 | MIT | HTML5 backend for React DnD |
| react-dom | 18.1.0 | MIT | React package for working with the DOM. |
| react-i18next | 11.18.3 | MIT | Internationalization for react done right. Using the i18next i18n ecosystem. |
| react-js-cron | ^5.0.1 | MIT | A React cron editor with antd inspired by jqCron |
| react-redux | 8.1.2 | MIT | Official React bindings for Redux |
| react-router | ^6.1.0 | MIT | Declarative routing for React |
| react-router-dom | ^6.1.0 | MIT | Declarative routing for React web applications |
| react-scripts | 5.0.1 | MIT | Configuration and scripts for Create React App. |
| sql.js | ^1.13.0 | MIT | SQLite library with support for opening and writing databases, prepared statements, and more. This SQLite library is in pure javascript (compiled with emscripten). |
| stream-browserify | ^3.0.0 | MIT | the stream module from node core for browsers |
| tinymce | ^7.0.0 | GPL-2.0-or-later | Web based JavaScript HTML WYSIWYG editor control. |
| util | ^0.12.5 | MIT | Node.js's util module for all engines |
| uuid | 9.0.0 | MIT | RFC9562 UUIDs |
| web-vitals | ^2.1.0 | Apache-2.0 | Easily measure performance metrics in JavaScript |

## B3

B3 is a Rust-based component providing cryptographic utilities and core functionality.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| anyhow | 1.0 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| base64 | 0.22 | MIT OR Apache-2.0 | encodes and decodes base64 as bytes or utf8 |
| bb8-postgres | 0.9 | MIT | Full-featured async (tokio-based) postgres connection pool (like r2d2) |
| borsh | 1.5 | MIT OR Apache-2.0 | Binary Object Representation Serializer for Hashing |
| cfg-if | 1.0 | MIT OR Apache-2.0 | A macro to ergonomically define an item depending on a large number of #[cfg] parameters. Structured like an if-else chain, the first matching branch is the item that gets emitted. |
| clap | 4.0 | MIT OR Apache-2.0 | A simple to use, efficient, and full-featured Command Line Argument Parser |
| config | 0.15 | MIT OR Apache-2.0 | Layered configuration system for Rust applications. |
| futures | 0.3 | MIT OR Apache-2.0 | An implementation of futures and streams featuring zero allocations, composability, and iterator-like interfaces. |
| hex | 0.4 | MIT OR Apache-2.0 | Encoding and decoding data into/from hexadecimal representation. |
| log | 0.4 | MIT OR Apache-2.0 | A lightweight logging facade for Rust |
| prost | 0.13 | Apache-2.0 | A Protocol Buffers implementation for the Rust Language. |
| rayon | 1.5 | MIT OR Apache-2.0 | Simple work-stealing parallelism for Rust |
| serde | 1.0 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| strum | 0.27 | MIT | Helpful macros for working with enums and strings |
| tokio | 1.38 | MIT | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications. |
| tokio-postgres | 0.7 | MIT OR Apache-2.0 | A native, asynchronous PostgreSQL client |
| toml | 0.8 | MIT OR Apache-2.0 | A native Rust encoder and decoder of TOML-formatted files and streams. Provides implementations of the standard Serialize/Deserialize traits for TOML data to facilitate deserializing and serializing Rust structures. |
| tonic | 0.13 | MIT | A gRPC over HTTP/2 implementation focused on high performance, interoperability, and flexibility. |
| tracing | 0.1 | MIT | Application-level tracing for Rust. |
| tracing-attributes | 0.1 | MIT | Procedural macro attributes for automatically instrumenting functions. |
| tracing-log | 0.2 | MIT | Provides compatibility between `tracing` and the `log` crate. |
| tracing-subscriber | 0.3 | MIT | Utilities for implementing and composing `tracing` subscribers. |
| tracing-tree | 0.4 | MIT OR Apache-2.0 | A Tracing Layer which prints a tree of spans and events. |

## Ballot Verifier

The ballot verifier is a React-based application for verifying ballot integrity and authenticity.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| @apollo/client | 3.8.0 | MIT | A fully-featured caching GraphQL client. |
| @craco/craco | ^7.1.0 | Apache-2.0 | Create React App Configuration Override, an easy and comprehensible configuration layer for create-react-app. |
| @emotion/react | 11.10.0 | MIT | > Simple styling in React. |
| @emotion/styled | 11.10.0 | MIT | styled API for emotion |
| @fortawesome/fontawesome-svg-core | ^6.1.1 | MIT | The iconic font, CSS, and SVG framework |
| @fortawesome/free-brands-svg-icons | ^6.1.1 | (CC-BY-4.0 AND MIT) | The iconic font, CSS, and SVG framework |
| @fortawesome/free-solid-svg-icons | ^6.1.1 | (CC-BY-4.0 AND MIT) | The iconic font, CSS, and SVG framework |
| @fortawesome/react-fontawesome | ^0.1.18 | MIT | Official React component for Font Awesome |
| @mui/material | 5.13.3 | MIT | Material UI is an open-source React component library that implements Google's Material Design. It's comprehensive and can be used in production out of the box. |
| @reduxjs/toolkit | 1.9.5 | MIT | The official, opinionated, batteries-included toolset for efficient Redux development |
| i18next | ^21.8.16 | MIT | i18next internationalization framework |
| i18next-browser-languagedetector | ^6.1.4 | MIT | language detector used in browser environment for i18next |
| lodash | ^4.17.21 | MIT | Lodash modular utilities. |
| mui-image | 1.0.7 | ISC | Display images as per the Material guidelines. For React apps using Material-UI. |
| react | 18.1.0 | MIT | React is a JavaScript library for building user interfaces. |
| react-dom | 18.1.0 | MIT | React package for working with the DOM. |
| react-i18next | 11.18.3 | MIT | Internationalization for react done right. Using the i18next i18n ecosystem. |
| react-redux | 8.1.2 | MIT | Official React bindings for Redux |
| react-router-dom | 6.11.2 | MIT | Declarative routing for React web applications |
| react-scripts | 5.0.1 | MIT | Configuration and scripts for Create React App. |
| web-vitals | ^2.1.0 | Apache-2.0 | Easily measure performance metrics in JavaScript |

## Braid

Braid is a Rust-based component providing consensus and distributed systems functionality.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| anyhow | 1.0 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| ascii_table | 4.0 | MIT | Print ASCII tables to the terminal |
| async-trait | 0.1 | MIT OR Apache-2.0 | Type erasure for async trait methods |
| base64 | 0.22 | MIT OR Apache-2.0 | encodes and decodes base64 as bytes or utf8 |
| cfg-if | 1.0 | MIT OR Apache-2.0 | A macro to ergonomically define an item depending on a large number of #[cfg] parameters. Structured like an if-else chain, the first matching branch is the item that gets emitted. |
| clap | 4.0 | MIT OR Apache-2.0 | A simple to use, efficient, and full-featured Command Line Argument Parser |
| colored | 3.0 | MPL-2.0 | The most simple way to add colors in your terminal |
| crepe | 0.1 | MIT OR Apache-2.0 | Datalog in Rust as a procedural macro |
| getrandom | =0.2 | MIT OR Apache-2.0 | A small cross-platform library for retrieving random data from system source |
| hex | 0.4 | MIT OR Apache-2.0 | Encoding and decoding data into/from hexadecimal representation. |
| log | 0.4 | MIT OR Apache-2.0 | A lightweight logging facade for Rust |
| rand | 0.9 | MIT OR Apache-2.0 | Random number generators and other randomness functionality. |
| rayon | 1.5 | MIT OR Apache-2.0 | Simple work-stealing parallelism for Rust |
| reedline-repl-rs | 1.0 | MIT | Library to generate a fancy REPL for your application based on reedline and clap |
| rusqlite | 0.32 | MIT | Ergonomic wrapper for SQLite |
| rustc-hash | 2.0 | Apache-2.0 OR MIT | A speedy, non-cryptographic hashing algorithm used by rustc |
| serde | 1.0 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| serde_json | 1.0 | MIT OR Apache-2.0 | A JSON serialization file format |
| strum | 0.27 | MIT | Helpful macros for working with enums and strings |
| thiserror | 2.0 | MIT OR Apache-2.0 | derive(Error) |
| tikv-jemalloc-ctl | 0.6 | MIT/Apache-2.0 | A safe wrapper over jemalloc's control and introspection APIs |
| tikv-jemallocator | 0.6 | MIT/Apache-2.0 | A Rust allocator backed by jemalloc |
| tokio | 1.40 | MIT | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications. |
| toml | 0.8 | MIT OR Apache-2.0 | A native Rust encoder and decoder of TOML-formatted files and streams. Provides implementations of the standard Serialize/Deserialize traits for TOML data to facilitate deserializing and serializing Rust structures. |
| tonic | 0.13 | MIT | A gRPC over HTTP/2 implementation focused on high performance, interoperability, and flexibility. |
| tracing | 0.1 | MIT | Application-level tracing for Rust. |
| tracing-attributes | 0.1 | MIT | Procedural macro attributes for automatically instrumenting functions. |
| tracing-log | 0.2 | MIT | Provides compatibility between `tracing` and the `log` crate. |
| tracing-subscriber | 0.3 | MIT | Utilities for implementing and composing `tracing` subscribers. |
| tracing-tree | 0.4 | MIT OR Apache-2.0 | A Tracing Layer which prints a tree of spans and events. |
| tracing-wasm | 0.2 | MIT OR Apache-2.0 | tracing subscriber for browser WASM |
| wasm-bindgen | =0.2 | MIT OR Apache-2.0 | Easy support for interacting between JS and Rust. |
| wasm-bindgen-rayon | 1.0 | Apache-2.0 | Adapter for using Rayon-based concurrency on the Web |

## E2e

E2E provides end-to-end testing capabilities and automation tools.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| anyhow | 1.0 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| clap | 4.0 | MIT OR Apache-2.0 | A simple to use, efficient, and full-featured Command Line Argument Parser |
| csv | 1.1 | Unlicense/MIT | Fast CSV parsing with support for serde. |
| rand | 0.9 | MIT OR Apache-2.0 | Random number generators and other randomness functionality. |
| reqwest | 0.12 | MIT OR Apache-2.0 | higher level HTTP client library |
| rocket | 0.5 | MIT OR Apache-2.0 | Web framework with a focus on usability, security, extensibility, and speed. |
| rusqlite | 0.32 | MIT | Ergonomic wrapper for SQLite |
| serde | 1.0 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| serde_json | 1.0 | MIT OR Apache-2.0 | A JSON serialization file format |
| tokio | 1.0 | MIT | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications. |
| uuid | 1.5 | Apache-2.0 OR MIT | A library to generate and parse UUIDs. |

## Electoral Log

Electoral Log provides comprehensive logging and auditing capabilities for electoral processes.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| anyhow | 1.0 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| borsh | 1.5 | MIT OR Apache-2.0 | Binary Object Representation Serializer for Hashing |
| clap | 4.0 | MIT OR Apache-2.0 | A simple to use, efficient, and full-featured Command Line Argument Parser |
| hex | 0.4 | MIT OR Apache-2.0 | Encoding and decoding data into/from hexadecimal representation. |
| log | 0.4 | MIT OR Apache-2.0 | A lightweight logging facade for Rust |
| serde | 1.0 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| serde_json | 1.0 | MIT OR Apache-2.0 | A JSON serialization file format |
| strum | 0.27 | MIT | Helpful macros for working with enums and strings |
| strum_macros | 0.27 | MIT | Helpful macros for working with enums and strings |
| tokio | 1.38 | MIT | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications. |
| tokio-stream | 0.1 | MIT | Utilities to work with `Stream` and `tokio`. |
| tonic | 0.13 | MIT | A gRPC over HTTP/2 implementation focused on high performance, interoperability, and flexibility. |
| tracing | 0.1 | MIT | Application-level tracing for Rust. |
| tracing-attributes | 0.1 | MIT | Procedural macro attributes for automatically instrumenting functions. |
| tracing-log | 0.2 | MIT | Provides compatibility between `tracing` and the `log` crate. |
| tracing-subscriber | 0.3 | MIT | Utilities for implementing and composing `tracing` subscribers. |
| tracing-tree | 0.4 | MIT OR Apache-2.0 | A Tracing Layer which prints a tree of spans and events. |

## Harvest

Harvest provides data collection and processing capabilities.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| anyhow | 1.0 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| aws-config | 1.0 | Apache-2.0 | AWS SDK config and credential provider implementations. |
| aws-sdk-s3 | 1.11 | Apache-2.0 | AWS SDK for Amazon Simple Storage Service |
| base64 | 0.22 | MIT OR Apache-2.0 | encodes and decodes base64 as bytes or utf8 |
| bstr | 1.11 | MIT OR Apache-2.0 | A string type that is not required to be valid UTF-8. |
| celery | 0.5 | Apache-2.0 | Rust implementation of Celery |
| chrono | 0.4 | MIT OR Apache-2.0 | Date and time library for Rust |
| deadpool-postgres | 0.14 | MIT OR Apache-2.0 | Dead simple async pool for tokio-postgres |
| dotenv | 0.15 | MIT | A `dotenv` implementation for Rust |
| either | 1.9 | MIT OR Apache-2.0 | The enum `Either` with variants `Left` and `Right` is a general purpose sum type with two cases. |
| graphql_client | 0.14 | Apache-2.0 OR MIT | Typed GraphQL requests and responses |
| handlebars | 6.1 | MIT | Handlebars templating implemented in Rust. |
| regex | 1.10 | MIT OR Apache-2.0 | An implementation of regular expressions for Rust. This implementation uses finite automata and guarantees linear time matching on all inputs. |
| reqwest | 0.12 | MIT OR Apache-2.0 | higher level HTTP client library |
| rocket | 0.5 | MIT OR Apache-2.0 | Web framework with a focus on usability, security, extensibility, and speed. |
| serde | 1.0 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| serde_json | 1.0 | MIT OR Apache-2.0 | A JSON serialization file format |
| serde_with | 3.4 | MIT OR Apache-2.0 | Custom de/serialization functions for Rust's serde |
| strum | 0.27 | MIT | Helpful macros for working with enums and strings |
| strum_macros | 0.27 | MIT | Helpful macros for working with enums and strings |
| tempfile | 3.15 | MIT OR Apache-2.0 | A library for managing temporary files and directories. |
| tokio | 1.32 | MIT | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications. |
| tokio-postgres | 0.7 | MIT OR Apache-2.0 | A native, asynchronous PostgreSQL client |
| tracing | 0.1 | MIT | Application-level tracing for Rust. |
| uuid | 1.5 | Apache-2.0 OR MIT | A library to generate and parse UUIDs. |

## Immu Board

Immu Board provides immutable board management and verification capabilities.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| anyhow | 1.0 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| clap | 4.0 | MIT OR Apache-2.0 | A simple to use, efficient, and full-featured Command Line Argument Parser |
| hex | 0.4 | MIT OR Apache-2.0 | Encoding and decoding data into/from hexadecimal representation. |
| log | 0.4 | MIT OR Apache-2.0 | A lightweight logging facade for Rust |
| tokio | 1.31 | MIT | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications. |
| tonic | 0.13 | MIT | A gRPC over HTTP/2 implementation focused on high performance, interoperability, and flexibility. |
| tracing | 0.1 | MIT | Application-level tracing for Rust. |
| tracing-attributes | 0.1 | MIT | Procedural macro attributes for automatically instrumenting functions. |
| tracing-log | 0.2 | MIT | Provides compatibility between `tracing` and the `log` crate. |
| tracing-subscriber | 0.3 | MIT | Utilities for implementing and composing `tracing` subscribers. |
| tracing-tree | 0.4 | MIT OR Apache-2.0 | A Tracing Layer which prints a tree of spans and events. |

## ImmuDB-RS

ImmuDB-RS provides Rust bindings for ImmuDB database operations.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| anyhow | 1.0 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| log | 0.4 | MIT OR Apache-2.0 | A lightweight logging facade for Rust |
| prost | 0.13 | Apache-2.0 | A Protocol Buffers implementation for the Rust Language. |
| prost-types | 0.13 | Apache-2.0 | Prost definitions of Protocol Buffers well known types. |
| serde | 1.0 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| tonic | 0.13 | MIT | A gRPC over HTTP/2 implementation focused on high performance, interoperability, and flexibility. |
| tracing | 0.1 | MIT | Application-level tracing for Rust. |
| tracing-attributes | 0.1 | MIT | Procedural macro attributes for automatically instrumenting functions. |
| tracing-log | 0.2 | MIT | Provides compatibility between `tracing` and the `log` crate. |
| tracing-subscriber | 0.3 | MIT | Utilities for implementing and composing `tracing` subscribers. |
| tracing-tree | 0.4 | MIT OR Apache-2.0 | A Tracing Layer which prints a tree of spans and events. |

## Keycloak Extensions

Keycloak Extensions provides custom authentication and authorization extensions.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| com.sun.mail:javax.mail | 1.6.2 | N/A |  |
| jakarta.validation:jakarta.validation-api | 3.1.1 | Apache License 2.0 | Jakarta Validation API |
| org.apache.httpcomponents:httpcore | 4.4.16 | N/A | Apache HttpComponents Core (blocking I/O) |
| org.projectlombok:lombok | 1.18.40 | The MIT License | Spice up your java: Automatic Resource Management, automatic generation of getters, setters, equals, hashCode and toString, and more! |

## Orare

Orare provides procedural macro utilities and code generation capabilities.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| anyhow | 1.0 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| cfg-if | 1.0 | MIT OR Apache-2.0 | A macro to ergonomically define an item depending on a large number of #[cfg] parameters. Structured like an if-else chain, the first matching branch is the item that gets emitted. |
| clap | 4.5 | MIT OR Apache-2.0 | A simple to use, efficient, and full-featured Command Line Argument Parser |
| proc-macro2 | 1.0 | MIT OR Apache-2.0 | A substitute implementation of the compiler's `proc_macro` API to decouple token-based libraries from the procedural macro use case. |
| quote | 1.0 | MIT OR Apache-2.0 | Quasi-quoting macro quote!(...) |
| serde | 1.0 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| serde_json | 1.0 | MIT OR Apache-2.0 | A JSON serialization file format |
| syn | 2.0 | MIT OR Apache-2.0 | Parser for Rust source code |

## Sequent Core

Sequent Core provides the fundamental libraries and utilities for the Sequent Voting Platform.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| ammonia | 4.1 | MIT OR Apache-2.0 | HTML Sanitization |
| anyhow | 1.0 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| async-trait | 0.1 | MIT OR Apache-2.0 | Type erasure for async trait methods |
| aws-config | 1.6 | Apache-2.0 | AWS SDK config and credential provider implementations. |
| aws-sdk-s3 | 1.79 | Apache-2.0 | AWS SDK for Amazon Simple Storage Service |
| aws-sdk-sesv2 | 1.70 | Apache-2.0 | AWS SDK for Amazon Simple Email Service |
| aws-sdk-sns | 1.63 | Apache-2.0 | AWS SDK for Amazon Simple Notification Service |
| aws-smithy-types | 1.3 | Apache-2.0 | Types for smithy-rs codegen. |
| base64 | 0.22 | MIT OR Apache-2.0 | encodes and decodes base64 as bytes or utf8 |
| borsh | 1.5 | MIT OR Apache-2.0 | Binary Object Representation Serializer for Hashing |
| cfg-if | 1.0 | MIT OR Apache-2.0 | A macro to ergonomically define an item depending on a large number of #[cfg] parameters. Structured like an if-else chain, the first matching branch is the item that gets emitted. |
| chrono | 0.4 | MIT OR Apache-2.0 | Date and time library for Rust |
| console_error_panic_hook | 0.1 | Apache-2.0/MIT | A panic hook for `wasm32-unknown-unknown` that logs panics to `console.error` |
| csv | 1.3.0 | Unlicense/MIT | Fast CSV parsing with support for serde. |
| curve25519-dalek | =4.1 | BSD-3-Clause | A pure-Rust implementation of group operations on ristretto255 and Curve25519 |
| ed25519-dalek | =2.1 | BSD-3-Clause | Fast and efficient ed25519 EdDSA key generations, signing, and verification in pure Rust. |
| handlebars | 6.1 | MIT | Handlebars templating implemented in Rust. |
| handlebars-chrono | 0.2 | BSD-2-Clause | Handlebars helper for using chrono DateTime |
| headless_chrome | 1.0 | MIT | Control Chrome programmatically |
| hex | 0.4 | MIT OR Apache-2.0 | Encoding and decoding data into/from hexadecimal representation. |
| jsonwebtoken | 9.1 | MIT | Create and decode JWTs in a strongly typed way. |
| keycloak | 24.0 | Unlicense OR MIT | Keycloak Admin REST API. |
| kuchiki | 0.8 | MIT | (朽木) HTML/XML tree manipulation library |
| num-bigint | 0.4 | MIT/Apache-2.0 | Big integer implementation for Rust |
| num-format | 0.4 | MIT/Apache-2.0 | A Rust crate for producing string-representations of numbers, formatted according to international standards |
| num-traits | 0.2 | MIT OR Apache-2.0 | Numeric traits for generic mathematics |
| openid | 0.17 | Unlicense OR MIT | OpenID Connect & Discovery client library using async / await. |
| ordered-float | 5.0 | MIT | Wrappers for total ordering on floats |
| phf | 0.11 | MIT | Runtime support for perfect hash function data structures |
| printpdf | 0.8 | MIT | Rust library for reading and writing PDF files |
| quick-error | 2.0 | MIT/Apache-2.0 | A macro which makes error types pleasant to write. |
| rand | =0.8 | MIT OR Apache-2.0 | Random number generators and other randomness functionality. |
| regex | 1.10 | MIT OR Apache-2.0 | An implementation of regular expressions for Rust. This implementation uses finite automata and guarantees linear time matching on all inputs. |
| reqwest | 0.12 | MIT OR Apache-2.0 | higher level HTTP client library |
| reqwest-middleware | 0.4 | MIT OR Apache-2.0 | Wrapper around reqwest to allow for client middleware chains. |
| reqwest-retry | 0.7 | MIT OR Apache-2.0 | Retry middleware for reqwest. |
| rocket | 0.5 | MIT OR Apache-2.0 | Web framework with a focus on usability, security, extensibility, and speed. |
| rusqlite | 0.32 | MIT | Ergonomic wrapper for SQLite |
| schemars | 0.8 | MIT | Generate JSON Schemas from Rust code |
| serde | 1.0 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| serde-wasm-bindgen | 0.6 | MIT | Native Serde adapter for wasm-bindgen |
| serde_json | 1.0 | MIT OR Apache-2.0 | A JSON serialization file format |
| serde_path_to_error | 0.1 | MIT OR Apache-2.0 | Path to the element that failed to deserialize |
| serde_urlencoded | 0.7 | MIT/Apache-2.0 | `x-www-form-urlencoded` meets Serde |
| sha256 | 1.6 | MIT OR Apache-2.0 | sha256 crypto digest |
| strum | 0.27 | MIT | Helpful macros for working with enums and strings |
| strum_macros | 0.27 | MIT | Helpful macros for working with enums and strings |
| tempfile | 3.15 | MIT OR Apache-2.0 | A library for managing temporary files and directories. |
| time | 0.3 | MIT OR Apache-2.0 | Date and time library. Fully interoperable with the standard library. Mostly compatible with #![no_std]. |
| tokio | 1.44 | MIT | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications. |
| tokio-postgres | 0.7 | MIT OR Apache-2.0 | A native, asynchronous PostgreSQL client |
| tracing | 0.1 | MIT | Application-level tracing for Rust. |
| tracing-log | 0.2 | MIT | Provides compatibility between `tracing` and the `log` crate. |
| tracing-subscriber | 0.3 | MIT | Utilities for implementing and composing `tracing` subscribers. |
| tracing-tree | 0.4 | MIT OR Apache-2.0 | A Tracing Layer which prints a tree of spans and events. |
| uuid | 1.5 | Apache-2.0 OR MIT | A library to generate and parse UUIDs. |
| warp | 0.3 | MIT | serve the web at warp speeds |
| wasm-bindgen | =0.2.100 | MIT OR Apache-2.0 | Easy support for interacting between JS and Rust. |
| web-sys | 0.3 | MIT OR Apache-2.0 | Bindings for all Web APIs, a procedurally generated crate from WebIDL |

## Step Cli

Step CLI provides command-line interface tools for managing the Sequent Voting Platform.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| anyhow | 1.0 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| base64 | 0.22 | MIT OR Apache-2.0 | encodes and decodes base64 as bytes or utf8 |
| chrono | 0.4 | MIT OR Apache-2.0 | Date and time library for Rust |
| clap | 4.5 | MIT OR Apache-2.0 | A simple to use, efficient, and full-featured Command Line Argument Parser |
| csv | 1.1 | Unlicense/MIT | Fast CSV parsing with support for serde. |
| deadpool-postgres | 0.14 | MIT OR Apache-2.0 | Dead simple async pool for tokio-postgres |
| fake | 4 | MIT OR Apache-2.0 | An easy to use library and command line for generating fake data like name, number, address, lorem, dates, etc. |
| graphql_client | 0.14 | Apache-2.0 OR MIT | Typed GraphQL requests and responses |
| hex | 0.4 | MIT OR Apache-2.0 | Encoding and decoding data into/from hexadecimal representation. |
| rand | 0.9 | MIT OR Apache-2.0 | Random number generators and other randomness functionality. |
| rayon | 1.5 | MIT OR Apache-2.0 | Simple work-stealing parallelism for Rust |
| reqwest | 0.12 | MIT OR Apache-2.0 | higher level HTTP client library |
| ring | 0.17 | Apache-2.0 AND ISC | An experiment. |
| serde | 1.0 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| serde_json | 1.0 | MIT OR Apache-2.0 | A JSON serialization file format |
| sha2 | 0.10 | MIT OR Apache-2.0 | Pure Rust implementation of the SHA-2 hash function family including SHA-224, SHA-256, SHA-384, and SHA-512. |
| strum | 0.27 | MIT | Helpful macros for working with enums and strings |
| strum_macros | 0.27 | MIT | Helpful macros for working with enums and strings |
| tokio | 1.38 | MIT | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications. |
| tokio-postgres | 0.7 | MIT OR Apache-2.0 | A native, asynchronous PostgreSQL client |
| uuid | 1.5 | Apache-2.0 OR MIT | A library to generate and parse UUIDs. |

## Strand

Strand provides cryptographic protocols and zero-knowledge proof capabilities.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| base64 | 0.22.1 | MIT OR Apache-2.0 | encodes and decodes base64 as bytes or utf8 |
| borsh | 1.5 | MIT OR Apache-2.0 | Binary Object Representation Serializer for Hashing |
| cfg-if | 1.0 | MIT OR Apache-2.0 | A macro to ergonomically define an item depending on a large number of #[cfg] parameters. Structured like an if-else chain, the first matching branch is the item that gets emitted. |
| chacha20poly1305 | 0.10 | Apache-2.0 OR MIT | Pure Rust implementation of the ChaCha20Poly1305 Authenticated Encryption with Additional Data Cipher (RFC 8439) with optional architecture-specific hardware acceleration. Also contains implementations of the XChaCha20Poly1305 extended nonce variant of ChaCha20Poly1305, and the reduced-round ChaCha8Poly1305 and ChaCha12Poly1305 lightweight variants. |
| curve25519-dalek | =4.1 | BSD-3-Clause | A pure-Rust implementation of group operations on ristretto255 and Curve25519 |
| ecdsa | 0.16 | Apache-2.0 OR MIT | Pure Rust implementation of the Elliptic Curve Digital Signature Algorithm (ECDSA) as specified in FIPS 186-4 (Digital Signature Standard), providing RFC6979 deterministic signatures as well as support for added entropy |
| ed25519-dalek | =2.1 | BSD-3-Clause | Fast and efficient ed25519 EdDSA key generations, signing, and verification in pure Rust. |
| generic-array | =0.14 | MIT | Generic types implementing functionality of arrays |
| getrandom | =0.2 | MIT OR Apache-2.0 | A small cross-platform library for retrieving random data from system source |
| hex | 0.4 | MIT OR Apache-2.0 | Encoding and decoding data into/from hexadecimal representation. |
| malachite | 0.6 | LGPL-3.0-only | Arbitrary-precision arithmetic, with efficient algorithms partially derived from GMP, FLINT, and MPFR. |
| num-bigint | 0.4 | MIT/Apache-2.0 | Big integer implementation for Rust |
| num-integer | 0.1 | MIT OR Apache-2.0 | Integer traits and functions |
| num-modular | =0.5 | Apache-2.0 | Implementation of efficient integer division and modular arithmetic operations with generic number types. Supports various backends including num-bigint, etc.. |
| num-traits | 0.2 | MIT OR Apache-2.0 | Numeric traits for generic mathematics |
| openssl | 0.10 | Apache-2.0 | OpenSSL bindings |
| p384 | 0.13 | Apache-2.0 OR MIT | Pure Rust implementation of the NIST P-384 (a.k.a. secp384r1) elliptic curve as defined in SP 800-186 with support for ECDH, ECDSA signing/verification, and general purpose curve arithmetic support. |
| rand | =0.8 | MIT OR Apache-2.0 | Random number generators and other randomness functionality. |
| rand_core | 0.9 | MIT OR Apache-2.0 | Core random number generator traits and tools for implementation. |
| rayon | 1.5 | MIT OR Apache-2.0 | Simple work-stealing parallelism for Rust |
| rcgen | 0.11.3 | MIT OR Apache-2.0 | Rust X.509 certificate generator |
| rug | ~1.23 | LGPL-3.0+ | Arbitrary-precision integers, rational, floating-point and complex numbers based on GMP, MPFR and MPC. |
| serde | 1.0.215 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| serde-wasm-bindgen | 0.6 | MIT | Native Serde adapter for wasm-bindgen |
| sha2 | 0.10 | MIT OR Apache-2.0 | Pure Rust implementation of the SHA-2 hash function family including SHA-224, SHA-256, SHA-384, and SHA-512. |
| sha3 | 0.10 | MIT OR Apache-2.0 | Pure Rust implementation of SHA-3, a family of Keccak-based hash functions including the SHAKE family of eXtendable-Output Functions (XOFs), as well as the accelerated variant TurboSHAKE |
| thiserror | 2.0 | MIT OR Apache-2.0 | derive(Error) |
| wasm-bindgen | =0.2.100 | MIT OR Apache-2.0 | Easy support for interacting between JS and Rust. |
| wasm-bindgen-rayon | 1.3 | Apache-2.0 | Adapter for using Rayon-based concurrency on the Web |
| web-sys | 0.3 | MIT OR Apache-2.0 | Bindings for all Web APIs, a procedurally generated crate from WebIDL |
| x509-parser | 0.15.1 | MIT OR Apache-2.0 | Parser for the X.509 v3 format (RFC 5280 certificates) |

## UI Core

UI Core provides shared user interface components and utilities.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| html-react-parser | 4.2.0 | MIT | HTML to React parser. |
| i18next | ^21.8.16 | MIT | i18next internationalization framework |
| i18next-browser-languagedetector | ^6.1.4 | MIT | language detector used in browser environment for i18next |
| moderndash | ^3.7.3 | MIT | A Typescript-First utility library inspired by Lodash. Optimized for modern browsers. |
| qrcode.react | 3.1.0 | ISC | React component to generate QR codes |
| react-scripts | 5.0.1 | MIT | Configuration and scripts for Create React App. |
| sanitize-html | 2.12.1 | MIT | Clean up user-submitted HTML, preserving allowlisted elements and allowlisted attributes on a per-element basis |

## UI Essentials

UI Essentials provides essential user interface components and styling utilities.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| @fortawesome/fontawesome-svg-core | ^6.1.1 | MIT | The iconic font, CSS, and SVG framework |
| @fortawesome/free-brands-svg-icons | ^6.1.1 | (CC-BY-4.0 AND MIT) | The iconic font, CSS, and SVG framework |
| @fortawesome/free-solid-svg-icons | ^6.1.1 | (CC-BY-4.0 AND MIT) | The iconic font, CSS, and SVG framework |
| @fortawesome/react-fontawesome | ^0.1.18 | MIT | Official React component for Font Awesome |
| i18next | ^21.8.16 | MIT | i18next internationalization framework |
| i18next-browser-languagedetector | ^6.1.4 | MIT | language detector used in browser environment for i18next |

## Velvet

Velvet provides verification and validation tools for electoral processes.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| anyhow | 1 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| clap | 4.4 | MIT OR Apache-2.0 | A simple to use, efficient, and full-featured Command Line Argument Parser |
| criterion | 0.5 | Apache-2.0 OR MIT | Statistics-driven micro-benchmarking library |
| csv | 1.3 | Unlicense/MIT | Fast CSV parsing with support for serde. |
| hex | 0.4 | MIT OR Apache-2.0 | Encoding and decoding data into/from hexadecimal representation. |
| num-bigint | 0.4 | MIT/Apache-2.0 | Big integer implementation for Rust |
| ordered-float | 4.4.0 | MIT | Wrappers for total ordering on floats |
| rand | 0.9 | MIT OR Apache-2.0 | Random number generators and other randomness functionality. |
| rayon | 1.8 | MIT OR Apache-2.0 | Simple work-stealing parallelism for Rust |
| rusqlite | 0.32 | MIT | Ergonomic wrapper for SQLite |
| serde | 1.0 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| serde_json | 1.0 | MIT OR Apache-2.0 | A JSON serialization file format |
| strum | 0.27 | MIT | Helpful macros for working with enums and strings |
| strum_macros | 0.27 | MIT | Helpful macros for working with enums and strings |
| tempfile | 3.15.0 | MIT OR Apache-2.0 | A library for managing temporary files and directories. |
| tokio | 1.32 | MIT | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications. |
| tracing | 0.1 | MIT | Application-level tracing for Rust. |
| uuid | 1.4 | Apache-2.0 OR MIT | A library to generate and parse UUIDs. |
| walkdir | 2 | Unlicense/MIT | Recursively walk a directory. |

## Voting Portal

The voting portal is a React-based web application that provides the voter interface for elections.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| @apollo/client | 4.0.5 | MIT | A fully-featured caching GraphQL client. |
| @emotion/react | 11.14.0 | MIT | > Simple styling in React. |
| @emotion/styled | 11.14.1 | MIT | styled API for emotion |
| @fortawesome/fontawesome-svg-core | ^7.0.1 | MIT | The iconic font, CSS, and SVG framework |
| @fortawesome/free-brands-svg-icons | ^7.0.1 | (CC-BY-4.0 AND MIT) | The iconic font, CSS, and SVG framework |
| @fortawesome/free-solid-svg-icons | ^7.0.1 | (CC-BY-4.0 AND MIT) | The iconic font, CSS, and SVG framework |
| @fortawesome/react-fontawesome | ^3.0.2 | MIT | Official React component for Font Awesome |
| @mui/material | 7.3.2 | MIT | Material UI is an open-source React component library that implements Google's Material Design. It's comprehensive and can be used in production out of the box. |
| @reduxjs/toolkit | 2.9.0 | MIT | The official, opinionated, batteries-included toolset for efficient Redux development |
| dotenv | 17.2.2 | BSD-2-Clause | Loads environment variables from .env file |
| graphql | 16.11.0 | MIT | A Query Language and Runtime which can target any service. |
| i18next | ^25.5.2 | MIT | i18next internationalization framework |
| i18next-browser-languagedetector | ^8.2.0 | MIT | language detector used in browser environment for i18next |
| keycloak-js | ^26.2.0 | Apache-2.0 | A client-side JavaScript OpenID Connect library that can be used to secure web applications. |
| lodash | ^4.17.21 | MIT | Lodash modular utilities. |
| mui-image | 1.0.9 | ISC | Display images as per the Material guidelines. For React apps using Material-UI. |
| react | 19.1.1 | MIT | React is a JavaScript library for building user interfaces. |
| react-dom | 19.1.1 | MIT | React package for working with the DOM. |
| react-i18next | 15.7.3 | MIT | Internationalization for react done right. Using the i18next i18n ecosystem. |
| react-redux | 9.2.0 | MIT | Official React bindings for Redux |
| react-router-dom | 7.9.1 | MIT | Declarative routing for React web applications |
| react-scripts | 5.0.1 | MIT | Configuration and scripts for Create React App. |
| uuid | 13.0.0 | MIT | RFC9562 UUIDs |
| web-vitals | ^5.1.0 | Apache-2.0 | Easily measure performance metrics in JavaScript |

## Windmill

Windmill provides workflow automation and task orchestration capabilities.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| anyhow | 1.0 | MIT OR Apache-2.0 | Flexible concrete Error type built on std::error::Error |
| async-trait | 0.1 | MIT OR Apache-2.0 | Type erasure for async trait methods |
| async_once | 0.2 | MIT OR Apache-2.0 | async once tool for lazy_static |
| aws-config | 1.6 | Apache-2.0 | AWS SDK config and credential provider implementations. |
| aws-sdk-s3 | 1.79 | Apache-2.0 | AWS SDK for Amazon Simple Storage Service |
| aws-sdk-secretsmanager | 1.66 | Apache-2.0 | AWS SDK for AWS Secrets Manager |
| aws-sdk-sesv2 | 1.70 | Apache-2.0 | AWS SDK for Amazon Simple Email Service |
| aws-sdk-sns | 1.63 | Apache-2.0 | AWS SDK for Amazon Simple Notification Service |
| aws-smithy-types | 1.3 | Apache-2.0 | Types for smithy-rs codegen. |
| base64 | 0.22 | MIT OR Apache-2.0 | encodes and decodes base64 as bytes or utf8 |
| celery | N/A | Apache-2.0 | Rust implementation of Celery |
| cfg-if | 1.0 | MIT OR Apache-2.0 | A macro to ergonomically define an item depending on a large number of #[cfg] parameters. Structured like an if-else chain, the first matching branch is the item that gets emitted. |
| chrono | 0.4 | MIT OR Apache-2.0 | Date and time library for Rust |
| clap | 4.4 | MIT OR Apache-2.0 | A simple to use, efficient, and full-featured Command Line Argument Parser |
| config | 0.15 | MIT OR Apache-2.0 | Layered configuration system for Rust applications. |
| croner | 2.0 | MIT | Fully-featured, lightweight, and efficient Rust library designed for parsing and evaluating cron patterns |
| csv | 1.3 | Unlicense/MIT | Fast CSV parsing with support for serde. |
| deadpool | 0.12 | MIT OR Apache-2.0 | Dead simple async pool |
| deadpool-postgres | 0.14 | MIT OR Apache-2.0 | Dead simple async pool for tokio-postgres |
| dotenv | 0.15 | MIT | A `dotenv` implementation for Rust |
| ecies | 0.2.7 | MIT | Elliptic Curve Integrated Encryption Scheme for secp256k1 |
| encoding_rs | 0.8 | (Apache-2.0 OR MIT) AND BSD-3-Clause | A Gecko-oriented implementation of the Encoding Standard |
| encoding_rs_io | 0.1 | MIT OR Apache-2.0 | Streaming transcoding for encoding_rs |
| flate2 | 1.0 | MIT OR Apache-2.0 | DEFLATE compression and decompression exposed as Read/BufRead/Write streams. Supports miniz_oxide and multiple zlib implementations. Supports zlib, gzip, and raw deflate streams. |
| fs_extra | 1.3 | MIT | Expanding std::fs and std::io. Recursively copy folders with information about process and much more. |
| futures | 0.3 | MIT OR Apache-2.0 | An implementation of futures and streams featuring zero allocations, composability, and iterator-like interfaces. |
| google-calendar3 | 6.0.0 | MIT | A complete library to interact with calendar (protocol v3) |
| graphql_client | 0.14 | Apache-2.0 OR MIT | Typed GraphQL requests and responses |
| handlebars | 6.1 | MIT | Handlebars templating implemented in Rust. |
| hex | 0.4 | MIT OR Apache-2.0 | Encoding and decoding data into/from hexadecimal representation. |
| keycloak | 24.0 | Unlicense OR MIT | Keycloak Admin REST API. |
| lapin | 2.5 | MIT | AMQP client library |
| lazy_static | 1.4 | MIT OR Apache-2.0 | A macro for declaring lazily evaluated statics in Rust. |
| lettre | 0.11 | MIT | Email client |
| num_cpus | 1.16 | MIT OR Apache-2.0 | Get the number of CPUs on a machine. |
| once_cell | 1.20 | MIT OR Apache-2.0 | Single assignment cells and lazy values. |
| openssl | 0.10 | Apache-2.0 | OpenSSL bindings |
| ordered-float | 5.0 | MIT | Wrappers for total ordering on floats |
| postgres-openssl | 0.5 | MIT OR Apache-2.0 | TLS support for tokio-postgres via openssl |
| quick-error | 2.0 | MIT/Apache-2.0 | A macro which makes error types pleasant to write. |
| rand | 0.9 | MIT OR Apache-2.0 | Random number generators and other randomness functionality. |
| rayon | 1.5 | MIT OR Apache-2.0 | Simple work-stealing parallelism for Rust |
| regex | 1.10 | MIT OR Apache-2.0 | An implementation of regular expressions for Rust. This implementation uses finite automata and guarantees linear time matching on all inputs. |
| reqwest | 0.12 | MIT OR Apache-2.0 | higher level HTTP client library |
| ring | 0.17 | Apache-2.0 AND ISC | An experiment. |
| rocket | 0.5 | MIT OR Apache-2.0 | Web framework with a focus on usability, security, extensibility, and speed. |
| rusqlite | 0.32 | MIT | Ergonomic wrapper for SQLite |
| rust-s3 | 0.35 | MIT | Rust library for working with AWS S3 and compatible object storage APIs |
| rust_decimal | 1.36 | MIT | Decimal number implementation written in pure Rust suitable for financial and fixed-precision calculations. |
| rust_decimal_macros | 1.36 | MIT | Shorthand macros to assist creating Decimal types. |
| rust_xlsxwriter | 0.90.0 | MIT OR Apache-2.0 | A Rust library for writing Excel 2007 xlsx files |
| rustls | 0.23.32 | Apache-2.0 OR ISC OR MIT | Rustls is a modern TLS library written in Rust. |
| serde | 1.0 | MIT OR Apache-2.0 | A generic serialization/deserialization framework |
| serde_json | 1.0 | MIT OR Apache-2.0 | A JSON serialization file format |
| serde_path_to_error | 0.1 | MIT OR Apache-2.0 | Path to the element that failed to deserialize |
| sha2 | 0.10 | MIT OR Apache-2.0 | Pure Rust implementation of the SHA-2 hash function family including SHA-224, SHA-256, SHA-384, and SHA-512. |
| strum | 0.27 | MIT | Helpful macros for working with enums and strings |
| strum_macros | 0.27 | MIT | Helpful macros for working with enums and strings |
| tar | 0.4 | MIT OR Apache-2.0 | A Rust implementation of a TAR file reader and writer. This library does not currently handle compression, but it is abstract over all I/O readers and writers. Additionally, great lengths are taken to ensure that the entire contents are never required to be entirely resident in memory all at once. |
| tempfile | 3.15 | MIT OR Apache-2.0 | A library for managing temporary files and directories. |
| thiserror | 2.0 | MIT OR Apache-2.0 | derive(Error) |
| time | 0.3 | MIT OR Apache-2.0 | Date and time library. Fully interoperable with the standard library. Mostly compatible with #![no_std]. |
| tokio | 1.32 | MIT | An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications. |
| tokio-postgres | 0.7 | MIT OR Apache-2.0 | A native, asynchronous PostgreSQL client |
| tokio-stream | 0.1 | MIT | Utilities to work with `Stream` and `tokio`. |
| tokio-util | 0.7 | MIT | Additional utilities for working with Tokio. |
| toml | 0.8 | MIT OR Apache-2.0 | A native Rust encoder and decoder of TOML-formatted files and streams. Provides implementations of the standard Serialize/Deserialize traits for TOML data to facilitate deserializing and serializing Rust structures. |
| tracing | 0.1 | MIT | Application-level tracing for Rust. |
| tracing-subscriber | 0.3 | MIT | Utilities for implementing and composing `tracing` subscribers. |
| unicode-normalization | 0.1 | MIT/Apache-2.0 | This crate provides functions for normalization of Unicode strings, including Canonical and Compatible Decomposition and Recomposition, as described in Unicode Standard Annex #15. |
| uuid | 1.5 | Apache-2.0 OR MIT | A library to generate and parse UUIDs. |
| walkdir | 2.3 | Unlicense/MIT | Recursively walk a directory. |
| xz2 | 0.1 | MIT/Apache-2.0 | Rust bindings to liblzma providing Read/Write streams as well as low-level in-memory encoding/decoding. |
| zip | 2.1 | MIT | Library to support the reading and writing of zip files. |

## Wrap Map Err

Wrap Map Err provides procedural macros for error handling utilities.

| Dependency | Version | License | Description |
|------------|---------|---------|-------------|
| proc-macro2 | 1.0 | MIT OR Apache-2.0 | A substitute implementation of the compiler's `proc_macro` API to decouple token-based libraries from the procedural macro use case. |
| quote | 1.0 | MIT OR Apache-2.0 | Quasi-quoting macro quote!(...) |
| syn | 2.0 | MIT OR Apache-2.0 | Parser for Rust source code |
