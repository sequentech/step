# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
# Persistent local shell: no SSH orchestration, transcript or automatic acceptance.
set +x
set +o history
unset HISTFILE
umask 077
agd_source() {
    local _agd_status=0 _agd_options _agd_trap
    _agd_options=$(set +o)
    _agd_trap=$(trap -p ERR)
    set -E -o pipefail
    # Return from the sourced block, then restore options without losing its variables/cwd.
    trap '_agd_status=$?; if [[ ${FUNCNAME[0]} == agd_source ]]; then trap - ERR; eval "$_agd_options"; if [[ -n $_agd_trap ]]; then eval "$_agd_trap"; fi; printf "Step stopped (exit %s). Review the stated check before any retry.\n" "$_agd_status" >&2; fi; return "$_agd_status"' ERR
    source "$1"
    _agd_status=$?
    trap - ERR
    eval "$_agd_options"
    if [[ -n $_agd_trap ]]; then eval "$_agd_trap"; fi
    return "$_agd_status"
}
step() {
    local _agd_file _agd_status
    set +x
    _agd_file=$(python3 "$AGD_BUNDLE/runner.py" prepare "$1") || return
    printf 'Running %s in the %s session. Compare every result with the handbook.\n' "$1" "$AGD_ROLE"
    agd_source "$_agd_file"
    _agd_status=$?
    printf 'Command block ended (status %s); this is not an acceptance result.\n' "$_agd_status"
    return "$_agd_status"
}
show() { python3 "$AGD_BUNDLE/runner.py" show "$1"; }
steps() { python3 "$AGD_BUNDLE/runner.py" list; }
PS1='AGD[${AGD_ROLE}] \w # '
if [[ -n ${AGD_BUNDLE:-} ]]; then
    source /root/deploy/hosts.env || return
    set +x
    printf 'Local %s session. Use steps, show NAME, step NAME, and exit. No command output is recorded.\n' "$AGD_ROLE"
    printf 'Keep this session open for dependent commands; read the handbook between steps.\n'
fi
