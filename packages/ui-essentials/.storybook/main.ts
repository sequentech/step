import type { StorybookConfig } from '@storybook/react-vite';
import { mergeConfig } from 'vite';
import path from 'path';

const config: StorybookConfig = {
  stories: [
    '../src/**/*.mdx',
    '../src/**/*.stories.@(js|jsx|ts|tsx)',
  ],
  addons: [
    '@storybook/addon-links',
    '@storybook/addon-essentials',
    '@storybook/addon-interactions',
    '@storybook/addon-docs', // enables MDX
    'storybook-addon-remix-react-router',
    'storybook-addon-pseudo-states',
    '@storybook/addon-viewport',
  ],
  framework: {
    name: '@storybook/react-vite',
    options: {},
  },
  docs: {
    autodocs: true,
    // Enable MDX v2+ support
    defaultName: 'Docs',
  },
  // Fix internal imports + alias
  async viteFinal(config, { configType }) {
    // CORRECT: Hoisted to /workspaces/node_modules/@storybook
    const rootNodeModules = path.resolve(__dirname, '../../../node_modules/@storybook');

    return mergeConfig(config, {
      resolve: {
        alias: [
          // Top-level
          { find: 'storybook/preview-api', replacement: '@storybook/preview-api' },
          { find: 'storybook/test', replacement: '@storybook/test' },

          // === ALL KNOWN INTERNAL SUBPATHS (from your logs) ===
          { find: 'storybook/internal/core-events', replacement: path.resolve(rootNodeModules, 'core-events') },
          { find: 'storybook/internal/client-logger', replacement: path.resolve(rootNodeModules, 'client-logger') },
          { find: 'storybook/internal/preview-errors', replacement: path.resolve(rootNodeModules, 'preview/dist/preview-errors') },
          { find: 'storybook/internal/docs-tools', replacement: path.resolve(rootNodeModules, 'docs-tools') },
          { find: 'storybook/internal/channels', replacement: path.resolve(rootNodeModules, 'channels') },
          { find: 'storybook/internal/preview/runtime', replacement: path.resolve(rootNodeModules, 'preview/dist/runtime') },
          { find: 'storybook/internal/instrumenter', replacement: path.resolve(rootNodeModules, 'instrumenter') },
          { find: 'storybook/internal/actions', replacement: path.resolve(rootNodeModules, 'addon-actions') },
          { find: 'storybook/internal/backgrounds', replacement: path.resolve(rootNodeModules, 'addon-backgrounds') },
          { find: 'storybook/internal/highlight', replacement: path.resolve(rootNodeModules, 'addon-highlight') },
          { find: 'storybook/internal/links', replacement: path.resolve(rootNodeModules, 'addon-links') },
          { find: 'storybook/internal/viewport', replacement: path.resolve(rootNodeModules, 'addon-viewport') },

          // Your custom alias
          { find: '@root', replacement: path.resolve(__dirname, '../src') },
        ],
      },
      optimizeDeps: {
        // BLOCK ALL STORYBOOK INTERNALS FROM PRE-BUNDLING
        exclude: [
          'storybook',
          'storybook/internal',
          'storybook/internal/*',
          '@storybook/core-events',
          '@storybook/client-logger',
          '@storybook/preview-errors',
          '@storybook/docs-tools',
          '@storybook/channels',
          '@storybook/preview-api',
          '@storybook/test',
          '@storybook/instrumenter',
          '@storybook/addon-actions',
          '@storybook/addon-backgrounds',
          '@storybook/addon-highlight',
          '@storybook/addon-links',
          '@storybook/addon-viewport',
        ],
      },
      build: {
        sourcemap: configType === 'DEVELOPMENT',
      },
    });
  },
};

export default config;