// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useRef} from "react"
import {Editor} from "@tinymce/tinymce-react"

type Props = {
    initialValue?: string
    value?: string
    editorRef: any
    onEditorChange?: () => void
}

export default function MyEditor({value, initialValue, editorRef, onEditorChange}: Props) {
    return (
        <>
            <Editor
                tinymceScriptSrc={process.env.PUBLIC_URL + "/tinymce/tinymce.min.js"}
                onInit={(_evt, editor) => (editorRef.current = editor)}
                initialValue={initialValue}
                value={value}
                init={{
                    promotion: false,
                    branding: false,
                    height: 500,
                    menubar: false,
                    plugins: [
                        "advlist",
                        "autolink",
                        "lists",
                        "link",
                        "image",
                        "charmap",
                        "anchor",
                        "searchreplace",
                        "visualblocks",
                        "code",
                        "fullscreen",
                        "insertdatetime",
                        "media",
                        "table",
                        "preview",
                        "help",
                        "wordcount",
                    ],
                    toolbar:
                        "undo redo | blocks | " +
                        "bold italic forecolor | alignleft aligncenter " +
                        "alignright alignjustify | bullist numlist outdent indent | " +
                        "removeformat | help",
                    content_style:
                        "body { font-family:Helvetica,Arial,sans-serif; font-size:14px }",
                }}
                onEditorChange={onEditorChange}
            />
        </>
    )
}
