const fse = require("fs-extra")
const path = require("path")

const topDir = __dirname + "/../"
const rootDir = __dirname

fse.emptyDirSync(path.join(topDir, "public", "tinymce"))

fse.copySync(
    path.join(topDir, "node_modules", "tinymce"),
    path.join(rootDir, "public", "tinymce"),
    {
        overwrite: true,
    }
)
