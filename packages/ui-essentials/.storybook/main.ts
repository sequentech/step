// ui-essentials/.storybook/main.ts
import type { StorybookConfig } from '@storybook/react-vite';
import { mergeConfig } from 'vite';
import path from 'path';

const rootNodeModules = path.resolve(__dirname, '../../node_modules');

const config: StorybookConfig = {
  stories: ['../src/**/*.mdx', '../src/**/*.stories.@(js|jsx|ts|tsx)'],
  addons: [
    '@storybook/addon-links',
    '@storybook/addon-essentials',
    '@storybook/addon-interactions',
    '@storybook/addon-docs',
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
    defaultName: 'Docs',
  },

  async viteFinal(viteConfig, { configType }) {
    return mergeConfig(viteConfig, {
      resolve: {
        alias: [
          { find: '@root', replacement: path.resolve(__dirname, '../src') },
          { find: 'storybook/preview-api', replacement: `${rootNodeModules}/@storybook/preview-api/dist/index.mjs` },
          { find: 'storybook/test', replacement: `${rootNodeModules}/@storybook/test/dist/index.mjs` },
          { find: 'storybook/internal/csf', replacement: `${rootNodeModules}/@storybook/csf/dist/index.mjs` },
          { find: 'storybook/internal/core-events', replacement: `${rootNodeModules}/@storybook/core-events/dist/index.mjs` },
          { find: 'storybook/internal/channels', replacement: `${rootNodeModules}/@storybook/channels/dist/index.mjs` },
          { find: 'storybook/internal/client-logger', replacement: `${rootNodeModules}/@storybook/client-logger/dist/index.mjs` },
          { find: 'storybook/internal/docs-tools', replacement: `${rootNodeModules}/@storybook/docs-tools/dist/index.mjs` },
          { find: 'storybook/internal/instrumenter', replacement: `${rootNodeModules}/@storybook/instrumenter/dist/index.mjs` },
        ],
      },

      optimizeDeps: {
        // Force Vite to pre-bundle these from root node_modules
        include: [
          '@storybook/csf',
          '@storybook/preview-api',
          '@storybook/core-events',
          '@storybook/channels',
          '@storybook/client-logger',
          '@storybook/docs-tools',
          '@storybook/instrumenter',
        ],
        // Prevent Vite from trying to bundle virtual modules
        exclude: [
          'storybook/preview-api',
          'storybook/test',
          'storybook/internal/csf',
          'storybook/internal/core-events',
          'storybook/internal/channels',
          'storybook/internal/client-logger',
          'storybook/internal/docs-tools',
          'storybook/internal/instrumenter',
        ],
      },

      build: {
        sourcemap: configType === 'DEVELOPMENT',
      },
    });
  },
};

export default config;