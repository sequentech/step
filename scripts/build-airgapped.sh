#!/usr/bin/env bash

TODAY="$(date '+%Y-%m-%d')"
SCRIPT_PATH="$(cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P)"
PROJECT_ROOT=$(realpath "$SCRIPT_PATH/..")
AIRGAPPED_ARTIFACTS_ROOT="$PROJECT_ROOT/airgapped-artifacts"
IMAGE_ARTIFACTS_PATH="$AIRGAPPED_ARTIFACTS_ROOT/images"
DELIVERABLE_PATH="$AIRGAPPED_ARTIFACTS_ROOT/deliverable"
DELIVERABLE_TARBALL="$DELIVERABLE_PATH/$TODAY.tar"

info() {
    echo "(info) $1"
}

fatal() {
    echo "(fatal) $1"
    exit 1
}

all-images() {
    yq -r '.services[].image' < $SCRIPT_PATH/../.devcontainer/docker-compose.yml | grep -vw null | sort | uniq
}

# Instead of doing some JSON/YAML golfing now, hardcode the images
# that are mounting directories from the root project so that we can
# repack them by adding these directories onto the image.
all-images-with-volumes() {
    cat <<EOF
sequentech.local/frontend ./packages /usr/src/app
sequentech.local/cargo-packages ./keycloak/import /import
sequentech.local/cargo-packages ./packages /app
EOF
}

# Re-tag the image, but with the host paths added to it. This is
# useful for creating air-gapped images that originally mounted
# directories such as the source code.
add-project-root-to-image() {
    if [ -e "$2" ]; then
        info "Copying $(realpath "$PROJECT_ROOT/$2") to $1 at $3; this will get $1 retagged"
        docker build -f- -t "$1" $PROJECT_ROOT <<EOF
          FROM $1
          COPY $2 $3
EOF
    fi
}

filesystem-friendly-image-name() {
    echo "$1" | sed -r 's|[/:]|-|g'
}

archive-image-artifact() {
    # Base64 has characters such as '=' that are invalid in some
    # filesystems. Use Base32 instead; longer filenames but safer.
    local image_artifact_path="$IMAGE_ARTIFACTS_PATH/$(filesystem-friendly-image-name "$1").tar.gz"
    info "Archiving image artifact $1 into $image_artifact_path"
    # docker save "$1" | gzip -1 > $image_artifact_path
    echo "this is image $1" > $image_artifact_path
}

add-readme-to-tarball() {
    tmpdir=$(mktemp -d)
    cat <<EOF > $tmpdir/README.md
    # Welcome to Sequent air-gapped environment

    ## Requirements

    - Docker Desktop

    ## Instructions

    In order to execute the system, you have to run the following command:

    ```shell-session
    $ ./up
    ```

    Once that it has been imported and started, you can visit the different services at their endpoints:

    - Admin portal: http://localhost:3002
    - Voting portal: http://localhost:3000
EOF
    tar --append --file=$DELIVERABLE_TARBALL $tmpdir/README.md
}

add-up-script-to-tarball() {
    tmpdir=$(mktemp -d)
    cat <<'EOF' > $tmpdir/up
#!/usr/bin/env bash

TODO (ereslibre)
EOF
    chmod +x $tmpdir/up
    tar --append --file=$DELIVERABLE_TARBALL $tmpdir/up
}

add-images-to-tarball() {
    for image in $(find $IMAGE_ARTIFACTS_PATH -type f -name "*.tar.gz"); do
        tar --append --file=$DELIVERABLE_TARBALL $image
    done
}

clean-artifacts-root() {
    find $AIRGAPPED_ARTIFACTS_ROOT -mindepth 1 -not -name .gitkeep | xargs rm
}

tar -cf $DELIVERABLE_TARBALL -T /dev/null

# First, take all images that volume mount the project source code,
# and add it to the image, retagging the images
IFS='
'
for image in $(all-images-with-volumes); do
    IFS=' ' read -a fields <<< "$image"
    image_name="${fields[0]}"
    host_path="${fields[1]}"
    target_path="${fields[2]}"
    add-project-root-to-image "$image_name" "$host_path" "$target_path"
done

# Archive all images
for image in $(all-images); do
    archive-image-artifact "$image"
done

add-images-to-tarball
add-up-script-to-tarball
add-readme-to-tarball

info "Project root: $PROJECT_ROOT"
info "Air gapped artifacts location: $AIRGAPPED_ARTIFACTS_ROOT"
