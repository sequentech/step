import React, {useRef} from "react"
import {Editor} from "@tinymce/tinymce-react"

type Props = {
    initialValue?: string
    value?: string
    editorRef: any
    onEditorChange?: () => void
    dir?: "ltr" | "rtl"
}

export default function MyEditor({value, initialValue, editorRef, onEditorChange, dir}: Props) {
    return (
        <>
            <Editor
                tinymceScriptSrc={process.env.PUBLIC_URL + "/tinymce/tinymce.min.js"}
                onInit={(_evt, editor) => (editorRef.current = editor)}
                initialValue={initialValue}
                value={value}
                init={{
                    directionality: dir ?? "ltr",
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
