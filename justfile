set positional-arguments

@default:
    just --list

install: install-node install-web-demo
    cargo binstall cargo-edit

test-core:
    cargo test -qp mintyml --no-default-features
    cargo test -qp mintyml --all-features

test-cli:
    cargo test -qp mintyml-cli

@test-node: install-node (build-node-wasm "node")
    just do minty-wasm test-exec

@test: test-core test-cli test-node

@build-node-wasm VARIANT:
    just do minty-wasm build-wasm-{{ VARIANT }}

install-node:
    #!/usr/bin/env pwsh
    $ErrorActionPreference = 'Stop'
    Import-Module ./build-utils.psm1
    Build-NodeManifest
    just do minty-wasm install

@build-node-tsc: install-node
    just do minty-wasm build-tsc

@build-node: install-node (build-node-wasm "web") (build-node-wasm "node") build-node-tsc

@publish-node: build-node test-node
    just do minty-wasm publish-exec

@install-web-demo:
    just do web-demo install

@serve-web-demo: install-web-demo && (build-node-wasm "web") build-node-tsc
    just do web-demo serve &

@build-web-demo: (build-node-wasm "web") build-node-tsc install-web-demo
    just do web-demo webpack

build-cli:
    cargo build -q --release --manifest-path minty-cli/Cargo.toml

publish-packages:
    pwsh -c ' \
    Import-Module ./build-utils.psm1; \
    Publish-Packages -Publish; \
    '

publish-release:
    pwsh -c ' \
    Import-Module ./build-utils.psm1; \
    Publish-Release -Publish; \
    '

build-release:
    pwsh -c ' \
    Import-Module ./build-utils.psm1; \
    Build-Release; \
    '

update-readme:
    pwsh -c ' \
    Import-Module ./doc-utils.psm1; \
    Build-ReadmeDotMd; \
    Build-ExampleIntro; \
    Build-CliReadme; \
    '

act *ARGS:
    #!/usr/bin/env sh
    repo=$(git remote get-url origin | sed -E 's:^.*\W(\w+/\w+)\.git$:\1:')
    gh act \
        --env GH_TOKEN=$(gh auth token) \
        --env CROSS_CONTAINER_IN_CONTAINER=true \
        --local-repository $repo=./ \
        --privileged \
        "$@"


@do DIR CMD *ARGS:
    shift 2; just {{DIR}}/{{CMD}} "$@"
