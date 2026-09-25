.PHONY: run web check release
run:
	cargo run --bin hw-monitor-native
web:
	cargo run --bin hw-monitor-web
check:
	cargo check --all-targets
release:
	cargo build --release --bin hw-monitor-native
