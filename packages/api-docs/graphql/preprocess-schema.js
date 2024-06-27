const fs = require('fs');
const path = require('path');
const { parse, print, visit } = require('graphql');

// Read the SDL schema file
const schemaFile = path.resolve(__dirname, 'docs/schema.graphql');
const outputSchemaFile = path.resolve(__dirname, 'docs/schema-no-schema-names.graphql');

// Load the schema as a string
const schemaSDL = fs.readFileSync(schemaFile, 'utf-8');

// Parse the schema to AST
const schemaAST = parse(schemaSDL);

// Helper function to remove prefix from names
const removePrefix = (name, prefix) => name.startsWith(prefix) ? name.replace(prefix, '') : name;

// Prefix to remove
const prefix = 'sequent_backend_';

// Function to remove schema names from types
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

// Process the schema AST
const processedAST = removeSchemaNames(schemaAST);

// Convert the modified AST back to SDL
const processedSDL = print(processedAST);

// Write the processed SDL to a new file
fs.writeFileSync(outputSchemaFile, processedSDL);