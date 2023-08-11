// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from 'react';
import {Redirect} from '@docusaurus/router';

export default function Home(): JSX.Element {
  return <Redirect to="docs/index" />;
};
