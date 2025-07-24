RUST_STABLE := trim(read("rust-stable-version"))
RUST_NIGTHLY := trim(read("rust-nightly-version"))

devs:
    rustup toolchain install {{ RUST_STABLE }} --no-self-update --component clippy,rustfmt
    rustup toolchain install {{ RUST_NIGTHLY }} --no-self-update --component clippy,rustfmt

toolchains:
    @echo {{ RUST_STABLE }}
    @echo {{ RUST_NIGTHLY }}

build *BIN:
    cargo +{{ RUST_STABLE }} build --locked {{ if BIN != "" { prepend("--bin ", BIN) } else { "--all" } }}

check:
    cargo +{{ RUST_STABLE }} check --all

fmt:
    cargo +{{ RUST_NIGTHLY }} fmt --all

fmt-check:
    cargo +{{ RUST_NIGTHLY }} fmt --all --check

taplo:
    taplo fmt

test:
    cargo +{{ RUST_STABLE }} test

clippy:
    cargo +{{ RUST_STABLE }} clippy

clippy-fix:
    cargo +{{ RUST_STABLE }} clippy --all --fix --allow-dirty --allow-staged

docker-up:
    docker compose up

docker-up-d:
    docker compose up -d

docker-dev-up:
    docker compose -f docker-compose-dev.yml --profile '*' up

docker-dev-up-db:
    docker compose -f docker-compose-dev.yml --profile db up

clean:
    cargo +{{ RUST_STABLE }} clean

run-chain:
    (cd chain && ./run.sh)

test-chain:
    (cd chain && ./run-test.sh)

run-governance:
    (cd governance && ./run.sh)

run-parameters:
    (cd parameters && ./run.sh)

run-pos:
    (cd pos && ./run.sh)

run-rewards:
    (cd rewards && ./run.sh)

run-seeder:
    (cd seeder && ./run.sh)

run-transactions:
    (cd transactions && ./run.sh)

run-webserver:
    (cd webserver && ./run.sh)
