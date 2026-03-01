.PHONY: build release install uninstall test test-integration lint check clean

INSTALL_DIR ?= $(HOME)/.local/bin
BINARY = lynx4ai

build:
	cargo build

release:
	cargo build --release

install: release
	@mkdir -p $(INSTALL_DIR)
	cp target/release/$(BINARY) $(INSTALL_DIR)/$(BINARY)
	chmod +x $(INSTALL_DIR)/$(BINARY)
	@echo "Installed: $(INSTALL_DIR)/$(BINARY)"
	@echo ""
	@echo "Add to Claude Code:  claude mcp add lynx4ai $(INSTALL_DIR)/$(BINARY)"

uninstall:
	rm -f $(INSTALL_DIR)/$(BINARY)
	@echo "Removed: $(INSTALL_DIR)/$(BINARY)"

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
