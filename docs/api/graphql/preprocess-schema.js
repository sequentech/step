// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const fs = require('fs');
const path = require('path');
const { buildClientSchema, printSchema, visit, parse, print } = require('graphql');

const schemaFile = path.resolve(__dirname, '../../../packages/graphql.schema.json');
const outputSchemaFile = path.resolve(__dirname, './docs/processed-schema.graphql');

const docsDirectory = path.dirname(outputSchemaFile);
if (!fs.existsSync(docsDirectory)) {
  console.log("Creating directory");
  fs.mkdirSync(docsDirectory, { recursive: true });
}
const schemaJSON = JSON.parse(fs.readFileSync(schemaFile, 'utf-8'));

const schema = buildClientSchema(schemaJSON);

const schemaSDL = printSchema(schema);

const schemaAST = parse(schemaSDL);

const removePrefix = (name, prefix) => name.startsWith(prefix) ? name.replace(prefix, '') : name;

const prefix = 'sequent_backend_';

const removeSchemaNames = (ast) => {
  return visit(ast, {
    ObjectTypeDefinition(node) {
      return {
        ...node,
        name: {
          ...node.name,
          value: removePrefix(node.name.value, prefix),
        },
      };
    },
    FieldDefinition(node) {
      if (node.type.name) {
        return {
          ...node,
          type: {
            ...node.type,
            name: {
              ...node.type.name,
              value: removePrefix(node.type.name.value, prefix),
            },
          },
        };
      }
      return undefined;
    },
    NamedType(node) {
      return {
        ...node,
        name: {
          ...node.name,
          value: removePrefix(node.name.value, prefix),
        },
      };
    },
    EnumTypeDefinition(node) {
      return {
        ...node,
        name: {
          ...node.name,
          value: removePrefix(node.name.value, prefix),
        },
      };
    },
    InputObjectTypeDefinition(node) {
      return {
        ...node,
        name: {
          ...node.name,
          value: removePrefix(node.name.value, prefix),
        },
      };
    },
    UnionTypeDefinition(node) {
      return {
        ...node,
        name: {
          ...node.name,
          value: removePrefix(node.name.value, prefix),
        },
      };
    },
    ScalarTypeDefinition(node) {
      return {
        ...node,
        name: {
          ...node.name,
          value: removePrefix(node.name.value, prefix),
        },
      };
    },
    InterfaceTypeDefinition(node) {
      return {
        ...node,
        name: {
          ...node.name,
          value: removePrefix(node.name.value, prefix),
        },
      };
    },
  });
};

const processedAST = removeSchemaNames(schemaAST);

const processedSDL = print(processedAST);

fs.writeFileSync(outputSchemaFile, processedSDL);