import type {CodegenConfig} from "@graphql-codegen/cli"

const config: CodegenConfig = {
    overwrite: true,
    schema: [
        {
            "http://graphql-engine:8080/v1/graphql": {
                headers: {
                    // "x-hasura-admin-secret": process.env.ADMIN_SECRET,
                },
            },
        },
    ],
    documents: "**/*.(graphql|ts|tsx)",
    generates: {
        "src/gql/": {
            preset: "client",
            plugins: [],
        },
        "./graphql.schema.json": {
            plugins: ["introspection"],
        },
    },
}

export default config
