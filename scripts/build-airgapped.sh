#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -x

if [[ -z "$STEP_VERSION" || -z "$STEP_HASH" ]]; then
    echo 'Export $STEP_VERSION envvar with the tagged version to package, and $STEP_HASH'
    exit 1
fi

TODAY="$(date '+%Y-%m-%d')"
SCRIPT_PATH="$(cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P)"
PROJECT_ROOT=$(realpath "$SCRIPT_PATH/..")
AIRGAPPED_ARTIFACTS_ROOT="$PROJECT_ROOT/airgapped-artifacts"
AIRGAPPED_ARTIFACTS_TODAY="$AIRGAPPED_ARTIFACTS_ROOT/$TODAY"
IMAGE_ARTIFACTS_PATH="$AIRGAPPED_ARTIFACTS_TODAY/images"
DELIVERABLE_TARBALL="$AIRGAPPED_ARTIFACTS_ROOT/$TODAY.tar"
export DOCKER_DEFAULT_PLATFORM=linux/amd64

info() {
    echo "(info) $1"
}

fatal() {
    echo "(fatal) $1"
    exit 1
}

# --- Scanovate (biometric) images: begin ---
# The vendor's four images, pinned by digest to the ones approved for the
# platform (GitOps f1205a4ef9aaadb329e63e10d777058ce298eb39
# docs/ovcs-scanovate/source-images.json: Sequent's ECR mirror copied
# unchanged, same digests). Nothing here pulls or builds them: SCANOVATE_OCI_DIR
# is an OCI image layout holding them, every blob checked against its digest.
# A full bundle carries all four whether or not a site starts them;
# ALLOW_MISSING_BIOMETRIC=1 makes a labelled DEVELOPER PARTIAL bundle without
# them (biometric.json "absent"), never the full offline bundle.
SCANOVATE_IMAGES=(
    "us-east1-docker.pkg.dev/sequent-production/sequent/scanovate/ocr@sha256:b305c844377682e33866cdca0b43a80249ef175697e885301f4026f0975c2246"
    "us-east1-docker.pkg.dev/sequent-production/sequent/scanovate/ngfacematch@sha256:13dadfa795f5b52667b488ad0d3d248dcf9f4bf825e5f44ff90436ddb8c178e7"
    "us-east1-docker.pkg.dev/sequent-production/sequent/scanovate/liveness-plus-service@sha256:75500fbeefefa3417ba38838daf222d6a9fb1460970df3c5da853e8813200b49"
    "us-east1-docker.pkg.dev/sequent-production/sequent/scanovate/liveness-presentation-detection-service@sha256:56d62785f400536085eaf6b4dfecb5fa42e462c95c2e2c43120b7c7a37fb501b"
)

# The linux/amd64 image of one digest: the digest itself, or an index's entry.
scanovate-manifest() {
    local dir="$1" digest="$2" blob manifest
    blob="$dir/blobs/sha256/${digest#sha256:}"
    [ -f "$blob" ] || { echo "(fatal) $digest is not in $dir" >&2; return 1; }
    if jq -e '.manifests' "$blob" >/dev/null 2>&1; then
        manifest=$(jq -r '[.manifests[] | select(.platform.os == "linux" and .platform.architecture == "amd64")][0].digest // empty' "$blob")
        [ -n "$manifest" ] || { echo "(fatal) $digest has no linux/amd64 image" >&2; return 1; }
        [ -f "$dir/blobs/sha256/${manifest#sha256:}" ] || { echo "(fatal) $manifest is not in $dir" >&2; return 1; }
        echo "$manifest"
    else
        echo "$digest"
    fi
}

# Every blob of one digest: itself, its platform manifest, config and layers.
scanovate-blobs() {
    local dir="$1" digest="$2" manifest
    manifest=$(scanovate-manifest "$dir" "$digest") || return 1
    [ "$manifest" = "$digest" ] || echo "$digest"
    echo "$manifest"
    jq -r '.config.digest, .layers[].digest' "$dir/blobs/sha256/${manifest#sha256:}"
}

# Verify SCANOVATE_OCI_DIR and set BIOMETRIC to "included", or to "absent" for
# an explicit developer partial bundle; anything else stops the build.
verify-scanovate-images() {
    BIOMETRIC="absent"
    if [ -n "${SCANOVATE_OCI_DIR:-}" ]; then
        info "Verifying the Scanovate images in $SCANOVATE_OCI_DIR"
        [ -f "$SCANOVATE_OCI_DIR/oci-layout" ] && [ -f "$SCANOVATE_OCI_DIR/index.json" ] \
            || { echo "(fatal) SCANOVATE_OCI_DIR=$SCANOVATE_OCI_DIR is not an OCI image layout" >&2; return 1; }
        local image blobs blob sums
        for image in "${SCANOVATE_IMAGES[@]}"; do
            blobs=$(scanovate-blobs "$SCANOVATE_OCI_DIR" "${image#*@}") || return 1
            sums=""
            while read -r blob; do
                [ -f "$SCANOVATE_OCI_DIR/blobs/sha256/${blob#sha256:}" ] \
                    || { echo "(fatal) $image needs $blob, which is not in $SCANOVATE_OCI_DIR" >&2; return 1; }
                sums+="${blob#sha256:}  $SCANOVATE_OCI_DIR/blobs/sha256/${blob#sha256:}"$'\n'
            done <<<"$blobs"
            printf '%s' "$sums" | sha256sum --check --quiet --strict \
                || { echo "(fatal) a blob of $image does not match its digest" >&2; return 1; }
            info "$image: verified"
        done
        BIOMETRIC="included"
    elif [ "${ALLOW_MISSING_BIOMETRIC:-0}" = "1" ]; then
        echo "WARNING: ALLOW_MISSING_BIOMETRIC=1: a DEVELOPER PARTIAL bundle without the Scanovate"
        echo "images (biometric.json \"absent\"). Biometric enrollment does not work with it, and it"
        echo "is not the full offline bundle."
    else
        {
            echo "(fatal) the full offline bundle carries Scanovate's images: set SCANOVATE_OCI_DIR to an"
            echo "        OCI layout holding the four pinned digests, or ALLOW_MISSING_BIOMETRIC=1 for a"
            echo "        labelled developer partial bundle without biometric enrollment."
        } >&2
        return 1
    fi
}

# One OCI archive of the four images, blobs unchanged and each named by its
# digest-pinned reference, which `docker load` keeps (containerd image store).
write-scanovate-archive() {
    local dest="$1" work image digest top type blob
    work=$(mktemp -d)
    mkdir -p "$work/blobs/sha256"
    printf '{"imageLayoutVersion":"1.0.0"}' > "$work/oci-layout"
    echo '{"schemaVersion":2,"mediaType":"application/vnd.oci.image.index.v1+json","manifests":[]}' > "$work/index.json"
    for image in "${SCANOVATE_IMAGES[@]}"; do
        digest="${image#*@}"
        for blob in $(scanovate-blobs "$SCANOVATE_OCI_DIR" "$digest"); do
            cp -n "$SCANOVATE_OCI_DIR/blobs/sha256/${blob#sha256:}" "$work/blobs/sha256/"
        done
        top="$SCANOVATE_OCI_DIR/blobs/sha256/${digest#sha256:}"
        type=$(jq -r '.mediaType // empty' "$top")
        [ -n "$type" ] || { echo "(fatal) $digest names no media type" >&2; return 1; }
        jq --arg ref "$image" --arg digest "$digest" --argjson size "$(stat -c %s "$top")" --arg type "$type" \
            '.manifests += [{mediaType: $type, digest: $digest, size: $size,
                annotations: {"io.containerd.image.name": $ref}}]' "$work/index.json" > "$work/index.new"
        mv "$work/index.new" "$work/index.json"
    done
    tar -C "$work" -cf "$dest" --sort=name --owner=0 --group=0 --numeric-owner oci-layout index.json blobs
    rm -rf "$work"
}
# --- Scanovate (biometric) images: end ---

docker-compose-airgap-preparation() {
    tmpfile=$(mktemp)
    sed "s/STEP_VERSION/$STEP_VERSION/g" $PROJECT_ROOT/.devcontainer/docker-compose-airgap-preparation.yml > $tmpfile
    echo $tmpfile
}

all-images() {
    yq -r '.services[].image' < $(docker-compose-airgap-preparation) | grep -vwE 'null|devenv' | sort | uniq
}

filesystem-friendly-image-name() {
    echo "$1" | sed -r 's|[/:]|-|g'
}

archive-image-artifact() {
    local image_name="$1"
    # Base64 has characters such as '=' that are invalid in some
    # filesystems. Use Base32 instead; longer filenames but safer.
    local image_artifact_path="$IMAGE_ARTIFACTS_PATH/$(filesystem-friendly-image-name "$image_name").tar"
    info "Archiving image artifact $image_name into $image_artifact_path"

    # Try to save the image
    if ! (docker save "$image_name" > "$image_artifact_path"); then
        echo "Image $image_name not found locally or failed to save. Attempting to pull the image..."
        # Try to pull the image
        if ! docker pull "$image_name"; then
            echo "Error: Failed to archive image artifact $image_name after failed pulling" >&2
        fi
        # Try to save the image again after pulling
        if ! (docker save "$image_name" > "$image_artifact_path"); then
            echo "Error: Failed to archive image artifact $image_name after pulling" >&2
        fi
    fi
}

build-images() {
    docker compose -f $PROJECT_ROOT/.devcontainer/docker-compose-airgap-preparation.yml --profile full build
}

pull-images() {
    docker compose -f $(docker-compose-airgap-preparation) --profile full pull --ignore-pull-failures
}

add-dotenv-to-tarball() {
    tmpdir=$(mktemp -d)
    cat $PROJECT_ROOT/scripts/airgap-files/.env | sed "s/STEP_VERSION/$STEP_VERSION/g" | sed "s/STEP_HASH/$STEP_HASH/g" > $tmpdir/.env

    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL .env
}


add-janitor-to-tarball() {
    JANITOR_PARENT="${PROJECT_ROOT}/packages/windmill/external-bin/"
    tar --append -C $JANITOR_PARENT --file=$DELIVERABLE_TARBALL janitor
}

add-database-init-to-tarball() {
    tmpdir=$(mktemp -d)
    mkdir -p $tmpdir/initdb
    cp "$PROJECT_ROOT/.devcontainer/postgresql/init-electoral-log.sh" "$tmpdir/initdb/20-electoral-log.sh"
    cp "$PROJECT_ROOT/.devcontainer/postgresql/init-task-queues.sh" "$tmpdir/initdb/30-task-queues.sh"
    cp "$PROJECT_ROOT/packages/electoral-log/catalog.sql" "$tmpdir/electoral-log-catalog.sql"
    tar --append -C "$tmpdir" --file="$DELIVERABLE_TARBALL" electoral-log-catalog.sql

    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL initdb
}

add-readme-to-tarball() {
    tmpdir=$(mktemp -d)
    cat $PROJECT_ROOT/scripts/airgap-files/README.md > $tmpdir/README.md

    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL README.md

}

add-docker-compose-to-tarball() {
    tmpdir=$(mktemp -d)
    cat $PROJECT_ROOT/scripts/airgap-files/docker-compose.yml | sed "s/STEP_VERSION/$STEP_VERSION/g" | sed "s/STEP_HASH/$STEP_HASH/g" > $tmpdir/docker-compose.yml

    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL docker-compose.yml
}

add-keycloak-data-to-tarball() {
    tmpdir=$(mktemp -d)
    mkdir -p $tmpdir/keycloak
    cp -r $PROJECT_ROOT/.devcontainer/keycloak/import $tmpdir/keycloak
    $PROJECT_ROOT/scripts/replacements.sh $PROJECT_ROOT/packages/windmill/external-bin/janitor/config/baseConfig.json $tmpdir/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL keycloak
}

add-hasura-data-to-tarball() {
    tmpdir=$(mktemp -d)
    mkdir -p $tmpdir/hasura
    cp -r $PROJECT_ROOT/hasura/{metadata,migrations} $tmpdir/hasura
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL hasura
}

add-minio-config-to-tarball() {
    tmpdir=$(mktemp -d)
    mkdir -p $tmpdir/minio
    cp -r $PROJECT_ROOT/.devcontainer/minio/nginx $tmpdir/minio/nginx
    cp -r $PROJECT_ROOT/.devcontainer/minio/public-assets $tmpdir/minio/public-assets
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL minio
}

# The admin portal's /b4/ forward to Harvest (mounted at /etc/nginx/default.d).
add-admin-portal-nginx-to-tarball() {
    tmpdir=$(mktemp -d)
    cp -r $PROJECT_ROOT/.devcontainer/admin-portal-nginx $tmpdir/admin-portal-nginx
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL admin-portal-nginx
}

# What the bundle carries for biometric enrollment, and the Liveness Plus
# configuration template `up` renders with the generated callback secret.
add-biometric-to-tarball() {
    tmpdir=$(mktemp -d)
    jq -n --arg biometric "$BIOMETRIC" --argjson images "$(printf '%s\n' "${SCANOVATE_IMAGES[@]}" | jq -R . | jq -s .)" \
        '{biometric: $biometric, images: (if $biometric == "included" then $images else [] end),
          approvedSource: "sequentech/gitops f1205a4ef9aaadb329e63e10d777058ce298eb39 docs/ovcs-scanovate/source-images.json"}' \
        > $tmpdir/biometric.json
    mkdir -p $tmpdir/scanovate/liveness
    cp $PROJECT_ROOT/scripts/airgap-files/scanovate/liveness/service_config.json.tmpl $tmpdir/scanovate/liveness/
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL biometric.json scanovate
}

add-up-script-to-tarball() {
    tmpdir=$(mktemp -d)
    cat $PROJECT_ROOT/scripts/airgap-files/up > $tmpdir/up

    chmod +x $tmpdir/up
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL up
}

add-images-to-tarball() {
    tar --append -C $AIRGAPPED_ARTIFACTS_TODAY --file=$DELIVERABLE_TARBALL images
}

clean-artifacts-root() {
    rm -rf $AIRGAPPED_ARTIFACTS_TODAY
}

# Before anything is built: a full bundle without its biometric images is
# refused here, not after hours of building.
verify-scanovate-images || exit 1

mkdir -p $DELIVERABLE_PATH $IMAGE_ARTIFACTS_PATH
tar -cf $DELIVERABLE_TARBALL -T /dev/null

build-images
pull-images

# Archive all images
for image in $(all-images); do
    archive-image-artifact "$image"
done

if [ "$BIOMETRIC" = "included" ]; then
    write-scanovate-archive "$IMAGE_ARTIFACTS_PATH/scanovate-oci.tar" || exit 1
fi
add-images-to-tarball
add-biometric-to-tarball
add-dotenv-to-tarball
add-minio-config-to-tarball
add-admin-portal-nginx-to-tarball
add-keycloak-data-to-tarball
add-docker-compose-to-tarball
add-hasura-data-to-tarball
add-up-script-to-tarball
add-readme-to-tarball
add-janitor-to-tarball

clean-artifacts-root

info "Project root: $PROJECT_ROOT"
info "Air gapped artifacts location: $AIRGAPPED_ARTIFACTS_ROOT"
