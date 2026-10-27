# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
(
set -euo pipefail
cd /root/deploy
D=$(mktemp -d /root/deploy/image-check.XXXXXX)
trap 'rm -rf "$D"' EXIT
blob() {
  local d=$1 f
  [[ $d =~ ^sha256:[0-9a-f]{64}$ ]] || return 1
  f=$D/${d#sha256:}
  [ -s "$f" ] || k3s ctr -n k8s.io content get "$d" > "$f" || return
  [ "$(sha256sum "$f" | cut -d' ' -f1)" = "${d#sha256:}" ] || return 1
  printf '%s\n' "$f"
}
config() {
  local d=$1 f c depth
  for depth in 1 2 3 4; do
    f=$(blob "$d") || return
    c=$(jq -r '.config.digest // empty' "$f") || return
    if [ -n "$c" ]; then
      blob "$c" > /dev/null || return
      jq -e '.layers | type=="array"' "$f" >/dev/null || return
      while IFS= read -r layer; do blob "$layer" >/dev/null || return; done < <(jq -er '.layers[].digest' "$f")
      printf '%s\n' "$c"
      return
    fi
    d=$(jq -er 'select((.manifests | length) == 1) | .manifests[0].digest' "$f") || return
  done
  return 1
}
grep -E '^(b4|ballot-verifier|braid|harvest|keycloak|windmill) sha256:' image-digests.txt > "$D/expected.txt"
[ "$(wc -l < "$D/expected.txt")" -eq 6 ]
[ "$(cut -d' ' -f1 "$D/expected.txt" | sort -u | wc -l)" -eq 6 ]
k3s kubectl get pods -A -o json > "$D/pods.json"
awk '{print "sequentech.local/" $1 "@" $2}' "$D/expected.txt" > "$D/refs.txt"
jq -e --rawfile refs "$D/refs.txt" '
  ($refs | split("\n") | map(select(length > 0))) as $expected
  | all(.items[]; . as $pod | all(.spec.containers[];
      if (.image | test("^sequentech\\.local/(b4|ballot-verifier|braid|harvest|keycloak|windmill)(@|:)"))
      then . as $container
        | (($expected | index($container.image)) != null)
        and ($pod.status.phase == "Succeeded" or any($pod.status.containerStatuses[]?;
          .name == $container.name and .ready == true and .state.running != null))
      else true end))' "$D/pods.json" > /dev/null
while read -r p d; do
  expected=$(config "$d")
  mapfile -t actual < <(jq -r --arg ref "sequentech.local/$p@$d" '
    .items[] as $pod | $pod.spec.containers[] | select(.image == $ref)
    | .name as $name | $pod.status.containerStatuses[]?
    | select(.name == $name and .ready == true and .state.running != null)
    | .imageID' "$D/pods.json")
  minimum=1; case "$p" in braid|windmill) minimum=2;; esac
  [ "${#actual[@]}" -ge "$minimum" ]
  for id in "${actual[@]}"; do
    observed=$(config "${id##*@}")
    [ "$observed" = "$expected" ]
  done
  printf '%s %s verified\n' "$p" "${#actual[@]}"
done < "$D/expected.txt"
if jq -r '.items[].spec.containers[].image' "$D/pods.json" \
  | grep -E 'sequentech\.local/(b4|ballot|braid|harvest|keycloak|windmill)' \
  | grep -v '@sha256:'; then
  echo 'FAIL: a TOE container uses an unpinned image' >&2
  exit 1
fi
)
