# 奇點世界 — 與 ./start 相同的「啟動前嚴格閘門」（Rust／cargo）。
# 用法：make verify

.PHONY: verify checkrooms build-server clippy test

verify: clippy test checkrooms
	@echo "verify OK（clippy + test + checkrooms -brackets -strict）"

clippy:
	cargo clippy -- -D warnings

test:
	cargo test

checkrooms:
	cargo run --bin checkrooms -- -brackets -strict

build-server:
	cargo build --release
	mkdir -p bin
	install -m 755 target/release/singularity_world bin/server-rust
