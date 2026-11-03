// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import { expect } from "storybook/test";

/** Storybook resets CLI viewport overrides; select and verify the actual story viewport. */
export const atViewport = (width: number) => ({
  globals: { viewport: { value: `width-${width}`, isRotated: false } },
  parameters: {
    viewport: {
      options: {
        [`width-${width}`]: {
          name: `${width}px`,
          styles: { width: `${width}px`, height: "1000px" },
        },
      },
    },
  },
  beforeEach: () => {
    expect(window.innerWidth, "Actual browser viewport width").toBe(width);
  },
});
