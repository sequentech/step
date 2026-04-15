---
id: rust_docs
title: Rust Docs
sidebar_position: -1
---

import useBaseUrl from '@docusaurus/useBaseUrl';

<!--
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

export const RustCrateLink = ({ path, children }) => {
  const url = useBaseUrl(path);
  return (
    <a href={url} target="_blank" rel="noreferrer noopener">
      {children}
    </a>
  );
};

### Crates

- <RustCrateLink path="/rust/sequent_core/index.html">sequent-core</RustCrateLink>
