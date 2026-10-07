// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import MDXComponents from "@theme-original/MDXComponents";
import Tabs from "@theme/Tabs";
import TabItem from "@theme/TabItem";

// Reuse the theme's keyboard navigation, persisted selection and code rendering.
export default { ...MDXComponents, CodeTabs: Tabs, CodeTab: TabItem };
