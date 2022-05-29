.PHONY: check
check:
	cargo check

.PHONY: clippy
clippy:
	cargo clippy

PHONY: test
test: unit-test

.PHONY: unit-test
unit-test:
	cargo unit-test

# Integration test
# .ONESHELL:
.PHONY: integration-test
integration-test: build _integration-test
_integration-test:
	@# this line below doesn't work, but the point is you need to use npm v16
	@#. ${HOME}/.nvm/nvm.sh && nvm use 16
	npm --prefix tests/ install
	npx ts-node ./tests/integration.ts

# This is a local build with debug-prints activated. Debug prints only show up
# in the local development chain (see the `start-server` command below)
# and mainnet won't accept contracts built with the feature enabled.
.PHONY: build _build
build: _build compress-wasm build-receiver
_build:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --features="debug-print"

# This is a build suitable for uploading to mainnet.
# Calls to `debug_print` get removed by the compiler.
.PHONY: build-mainnet _build-mainnet
build-mainnet: _build-mainnet compress-wasm
_build-mainnet:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown

# like build-mainnet, but slower and more deterministic
.PHONY: build-mainnet-reproducible
build-mainnet-reproducible:
	docker run --rm -v "$$(pwd)":/contract \
		--mount type=volume,source="$$(basename "$$(pwd)")_cache",target=/contract/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		enigmampc/secret-contract-optimizer:1.0.7

.PHONY: compress-wasm
compress-wasm:
	cp ./target/wasm32-unknown-unknown/release/*.wasm ./contract.wasm
	@## The following line is not necessary, may work only on linux (extra size optimization)
	@# wasm-opt -Os ./contract.wasm -o ./contract.wasm
	cat ./contract.wasm | gzip -9 > ./contract.wasm.gz

.PHONY: schema
schema:
	cargo run --example schema

# Ctrl-C to exit terminal, but does not stop the server
.PHONY: start-server
start-server:
	docker start -a localsecret || true 
	docker run -it -p 9091:9091 -p 26657:26657 -p 1317:1317 -p 5000:5000 --name localsecret ghcr.io/scrtlabs/localsecret

.PHONY: stop-server
stop-server:
	docker stop localsecret

.PHONY: reset-server
reset-server:
	docker stop localsecret || true
	docker rm localsecret || true
	docker run -it -p 9091:9091 -p 26657:26657 -p 1317:1317 -p 5000:5000 --name localsecret ghcr.io/scrtlabs/localsecret

# server needs to be running on another terminal
.PHONY: speedup-server
speedup-server:
	@# ok to reduce further to eg: 200ms
	docker exec localsecret sed -E -i '/timeout_(propose|prevote|precommit|commit)/s/[0-9]+m?s/500ms/' .secretd/config/config.toml
	docker stop localsecret
	docker start -a localsecret

.PHONY: clean
clean:
	cargo clean
	-rm -f ./contract.wasm ./contract.wasm.gz
	cd ./tests/example-receiver && $(MAKE) clean	

.PHONY: build-receiver
build-receiver:
	cd ./tests/example-receiver && $(MAKE) build