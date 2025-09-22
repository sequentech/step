// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export function downloadPDF(documentUrl: string, fileName?: string) {
    // Fetch the file from the URL
    fetch(documentUrl)
        .then((response) => response.blob())
        .then((blob) => {
            // Create a blob URL and download it
            const blobUrl = window.URL.createObjectURL(blob)
            const link = document.createElement("a")
            link.href = blobUrl
            link.setAttribute("download", fileName ?? "your-file.pdf") // Specify the file name
            document.body.appendChild(link)
            link.click()
            link.remove() // Clean up after the download
            window.URL.revokeObjectURL(blobUrl) // Free up memory
        })
        .catch((err) => console.error("Error downloading the file", err))
}
