.PHONY: build release test test-integration lint check clean

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

test-integration:
	cargo test -- --ignored

lint:
	cargo clippy -- -D warnings
	cargo fmt -- --check

check: lint test

clean:
	cargo clean
