export MAKEFLAGS        := "-j8"

cargo +args='':
    cargo {{args}}

check +args='':
    @just cargo check {{args}}

debug-build binary_name +args='':
    @just cargo build --bin {{binary_name}} {{args}}

release-build binary_name +args='':
    @just cargo build --bin {{binary_name}} --release {{args}}

example name +args='':
    @just cargo build --example {{name}} --features examples {{args}}

test +args='':
    @just cargo test {{args}}

# cargo doc --open
doc +args='':
    @just cargo doc --open {{args}}

# just rebuild docs, don't open browser page again
redoc +args='': 
    @just cargo doc {{args}}

# like doc, but include private items
doc-priv +args='':
    @just cargo doc --open --document-private-items {{args}}

bench +args='':
    @just cargo bench {{args}}

update +args='':
    @just cargo update {{args}}

# blow away build dir and start all over again
rebuild:
    just cargo clean
    just update
    just test

build-api-server-docs:
    just debug-build fitbod-server
    ./target/debug/fitbod-server --help > static/fitbod-server-main-help.txt
    ./target/debug/fitbod-server run --help > static/fitbod-server-run-help.txt
    ./target/debug/fitbod-server list-workouts-request --help > static/fitbod-server-list-workouts-request-help.txt
    ./target/debug/fitbod-server new-workouts-request --help > static/fitbod-server-new-workouts-request-help.txt
    ./target/debug/fitbod-server list-workouts-request > static/fitbod-server-list-workouts-request-http.txt
    ./target/debug/fitbod-server list-workouts-request --curl > static/fitbod-server-list-workouts-request-curl.txt
    just cargo run --bin generate-api-docs

# eval "$(./target/debug/fitbod-server list-workouts-request --curl) -s" | python3 -m json.tool > static/fitbod-server-list-workouts-request-curl-pretty-json.txt

show-users:
    psql -d fitbod -c 'select * from users limit 50;'
