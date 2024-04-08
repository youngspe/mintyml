set positional-arguments

@default:
    just --list

test-core:
    cargo test --manifest-path mintyml/Cargo.toml --verbose

test-cli:
    cargo test --manifest-path minty-cli/Cargo.toml --verbose

@test-node: (build-node-wasm "node")
    just exec minty-wasm test-exec

@test: test-core test-cli test-node

@build-node-wasm VARIANT: && build-node-tsc
    just exec minty-wasm build-wasm-{{ VARIANT }}

@build-node-tsc:
    just exec minty-wasm build-tsc

@build-node: (build-node-wasm "web") (build-node-wasm "node") && build-node-tsc

@publish-node: build-node test-node
    just exec minty-wasm publish-exec

@serve-web-demo: && (build-node-wasm "web") build-node-tsc
    just exec web-demo serve &

@build-web-demo: (build-node-wasm "web") build-node-tsc && fix-site-permissions
    just exec web-demo webpack

build-cli:
    cargo build --release --manifest-path minty-cli/Cargo.toml

fix-site-permissions:
    #!sh
    chmod -c -R +rX "web-demo/dist/" | while read line; do
        echo "::warning title=Invalid file permissions automatically fixed::$line"
    done

@exec DIR CMD *ARGS:
    just -f {{ DIR / 'justfile' }} "${@:2}"
