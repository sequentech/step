// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

// SPDX-License-Identifier: AGPL-3.0-only

import "@patternfly/react-core/dist/styles/base.css";
import { createRoot } from "react-dom/client";
import KeycloakManLovesJsx from "./KeycloakManLovesJsx";

const container = document.getElementById("app");
const root = createRoot(container!);

root.render(
  <KeycloakManLovesJsx />
);