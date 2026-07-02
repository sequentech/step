#!/bin/bash -i
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

if [ -z "$1" ]
then
    echo "Run: $0 <path/to/report_file.html>"
    exit 1
fi

[ -f "$1" ] && rm "$1"

#set -ex -o pipefail
# To find out the files with most lines of code of a specific extension, for
# example ".ts", to check if it should be excluded, you can execute:
# 
# find . | grep "\.ts$" | xargs wc -l 2>/dev/null | sort -r | head -n 10
#
# If you want to exclude a specific file, you can add it to `.ignore` located
# in the dir where `scc` is executing.

echo "# Lines of Code Report - LOCs" >> "$1"

for i in $(find packages -maxdepth 1 -type d)
do
    if [[ "$i" == "packages/rust-local-target" ]] || [[ "$i" == "packages/target" ]]
    then
        continue
    fi

    echo -e "\n## Directory: $i\n" >> "$1"
    JSON_FILE="/tmp/tmp_report"
    [ ! -f "${JSON_FILE}" ] || rm "${JSON_FILE}"

    scc \
        -s lines \
        --no-cocomo \
        --no-complexity \
        --format json "$i" >> ${JSON_FILE}
    
    # Use jq to parse JSON and format as markdown table
    cat ${JSON_FILE} | jq -r '
    . |
    [
        "Name", "Lines", "Code", "Comment", "Blank"
    ] | @tsv | gsub("\t"; " | ") |
    "| " + . + " |"
    ' >> "$1"
    echo "| --- | --- | --- | --- | --- |" >> "$1"

    cat ${JSON_FILE} | jq -r '
    def h:
      def hh: .[0] as $s | .[1] as $answer
        | if ($s|length) == 0 then $answer
          else ((if $answer == "" then "" else "," end) + $answer ) as $a
          | [$s[0:-3], $s[-3:] + $a] | hh
          end;
       [ tostring, ""] | hh;

    . | limit(5; .[]) | 
    [
        "**" + .Name + "**", (.Lines | h), (.Code | h), (.Comment | h), (.Blank | h)
    ] | @tsv | gsub("\t"; " | ") |
    "| " + . + " |"
    ' >> "$1"

    cat ${JSON_FILE} | jq -r '
    def h:
      def hh: .[0] as $s | .[1] as $answer
        | if ($s|length) == 0 then $answer
          else ((if $answer == "" then "" else "," end) + $answer ) as $a
          | [$s[0:-3], $s[-3:] + $a] | hh
          end;
       [ tostring, ""] | hh;

    "| **Total** | " + (
        [
        (. | map(.Lines) | add | h),
        (. | map(.Code) | add | h),
        (. | map(.Comment) | add | h),
        (. | map(.Blank) | add | h)
        ] | map(tostring) | join(" | ")
    ) + " |"
    ' >> "$1"
done
