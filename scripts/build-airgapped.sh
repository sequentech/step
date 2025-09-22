#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
    cat $PROJECT_ROOT/scripts/airgap-files/b3.sql > $tmpdir/initdb/b3.sql

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
    $PROJECT_ROOT/scripts/replacements.sh $PROJECT_ROOT/packages/windmill/external-bin/janitor/config/baseConfig.json $tmpdir/keycloak/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL keycloak
}

add-trustees-data-to-tarball() {
    tmpdir=$(mktemp -d)
    cp -r $PROJECT_ROOT/.devcontainer/trustees-data $tmpdir/trustees-data
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL trustees-data
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
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL minio
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

mkdir -p $DELIVERABLE_PATH $IMAGE_ARTIFACTS_PATH
tar -cf $DELIVERABLE_TARBALL -T /dev/null

build-images
pull-images

# Archive all images
for image in $(all-images); do
    archive-image-artifact "$image"
done

add-images-to-tarball
add-dotenv-to-tarball
add-minio-config-to-tarball
add-docker-compose-to-tarball
add-keycloak-data-to-tarball
add-trustees-data-to-tarball
add-hasura-data-to-tarball
add-up-script-to-tarball
add-database-init-to-tarball
add-readme-to-tarball
add-janitor-to-tarball

clean-artifacts-root

info "Project root: $PROJECT_ROOT"
info "Air gapped artifacts location: $AIRGAPPED_ARTIFACTS_ROOT"
