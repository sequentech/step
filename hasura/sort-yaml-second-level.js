import {glob} from "glob"
import fs from "fs"
import yaml from "js-yaml"

const filePattern = "metadata/databases/backend-db/tables/**/*.yaml"

// Helper function to determine if an object should have its keys sorted
const shouldSortKeys = (obj, level = 0) => {
    return (
        typeof obj === "object" &&
        obj !== null &&
        !Array.isArray(obj) &&
        level === 1
    ) // Only sort at level 1 (second level)
}

// Main sorting function
const sortYamlObject = (obj, level = 0) => {
    if (typeof obj !== "object" || obj === null) {
        return obj
    }

    // If it's an array, process each element but maintain array structure
    if (Array.isArray(obj)) {
        return obj.map((item) => sortYamlObject(item, level))
    }

    // Create new object with either sorted or original keys
    const keys = shouldSortKeys(obj, level)
        ? Object.keys(obj).sort()
        : Object.keys(obj)

    return keys.reduce((acc, key) => {
        // Recursively process nested objects/arrays with incremented level
        acc[key] = sortYamlObject(obj[key], level + 1)
        return acc
    }, {})
}

glob(filePattern)
    .then((files) => {
        files.forEach((file) => {
            try {
                const fileContent = fs.readFileSync(file, "utf8")
                const parsedYaml = yaml.load(fileContent)
                const sortedYaml = sortYamlObject(parsedYaml)

                const sortedContent = yaml.dump(sortedYaml, {
                    lineWidth: 80,
                    noRefs: true,
                    quotingType: '"',
                })

                fs.writeFileSync(file, sortedContent, "utf8")
                console.log(`✓ Sorted and updated: ${file}`)
            } catch (error) {
                console.error(`Error processing ${file}:`, error)
            }
        })
    })
    .catch((err) => {
        console.error("Error finding YAML files:", err)
        process.exit(1)
    })
