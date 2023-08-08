
import type { CodegenConfig } from '@graphql-codegen/cli';

const config: CodegenConfig = {
  overwrite: true,
  schema: "http://graphql-engine:8080/v1/graphql",
  documents: "**/*.graphql",
  generates: {
    "src/gql/": {
      preset: "client",
      plugins: [
        'typescript',
        'typescript-operations',
        'typescript-document-nodes',
        'typescript-react-apollo'
      ]
    },
    "./graphql.schema.json": {
      plugins: ["introspection"]
    }
  }
};

export default config;
