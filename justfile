set positional-arguments

@default:
    just --list

test-core:
    cargo test --manifest-path mintyml/Cargo.toml --verbose

test-cli:
    cargo test --manifest-path minty-cli/Cargo.toml --verbose

@test-node: install-node (build-node-wasm "node")
    just exec minty-wasm test-exec

@test: test-core test-cli test-node

@build-node-wasm VARIANT:
    just exec minty-wasm build-wasm-{{ VARIANT }}

@install-node:
    just exec minty-wasm install

@build-node-tsc: install-node
    just exec minty-wasm build-tsc

@build-node: install-node (build-node-wasm "web") (build-node-wasm "node") build-node-tsc

@publish-node: build-node test-node
    just exec minty-wasm publish-exec

@install-web-demo:
    just exec web-demo install

@serve-web-demo: install-web-demo && (build-node-wasm "web") build-node-tsc
    just exec web-demo serve &

@build-web-demo: (build-node-wasm "web") build-node-tsc install-web-demo
    just exec web-demo webpack

build-cli:
    cargo build --release --manifest-path minty-cli/Cargo.toml

act *ARGS:
    #!sh
    repo=$(git remote get-url origin | sed -E 's:^.*\W(\w+/\w+)\.git$:\1:')
    gh act --env GH_TOKEN=$(gh auth token) --pull=false --rebuild=false --local-repository $repo=./ "$@"

@exec DIR CMD *ARGS:
    shift 2; just {{DIR}}/{{CMD}} "$@"
