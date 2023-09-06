# runs schema, docs, unit-test, and clippy (incl on unit tests). 
# Doesn't do integration-test or mainnet-build 
.PHONY: prep
prep: schema doc test _clippy-test
_clippy-test:
	cargo clippy --tests

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
	RUST_BACKTRACE=1 cargo test

.PHONY: unit-test-nocapture
unit-test-nocapture:
	RUST_BACKTRACE=1 cargo test -- --nocapture

# Integration test
# .ONESHELL:
.PHONY: integration-test
integration-test: compile _integration-test
_integration-test:
	@# this line below doesn't work, but the point is you need to use npm v16
	@#. ${HOME}/.nvm/nvm.sh && nvm use 16
	npm --prefix tests/ install
	npx ts-node ./tests/integration.ts

.PHONY: compile _compile
compile: _compile contract.wasm.gz
_compile:
	cargo build --target wasm32-unknown-unknown --locked
	cp ./target/wasm32-unknown-unknown/debug/*.wasm ./contract.wasm
	@# The following line is not necessary, may work only on linux (extra size optimization)
	wasm-opt -Oz ./target/wasm32-unknown-unknown/release/*.wasm -o ./contract.wasm

.PHONY: compile-optimized _compile-optimized
compile-optimized: _compile-optimized contract.wasm.gz
_compile-optimized:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --locked
	@# The following line is not necessary, may work only on linux (extra size optimization)
	wasm-opt -Oz ./target/wasm32-unknown-unknown/release/*.wasm -o ./contract.wasm

.PHONY: compile-optimized-reproducible
compile-optimized-reproducible:
	docker run --rm -v "$$(pwd)":/contract \
		--mount type=volume,source="$$(basename "$$(pwd)")_cache",target=/code/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		enigmampc/secret-contract-optimizer:1.0.10

contract.wasm.gz: contract.wasm
	cat ./contract.wasm | gzip -9 > ./contract.wasm.gz

.PHONY: schema
schema:
	cargo run --example schema

# Ctrl-C to exit terminal, but does not stop the server
.PHONY: start-server
start-server:
	docker start -a localsecret || true 
	docker run -it -p 9091:9091 -p 26657:26657 -p 26656:26656 -p 1317:1317 -p 5000:5000 --name localsecret ghcr.io/scrtlabs/localsecret

.PHONY: stop-server
stop-server:
	docker stop localsecret

.PHONY: reset-server
reset-server:
	docker stop localsecret || true
	docker rm localsecret || true
	docker run -it -p 9091:9091 -p 26657:26657 -p 26656:26656 -p 1317:1317 -p 5000:5000 --name localsecret ghcr.io/scrtlabs/localsecret

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
	-rm -rf ./tests/node_modules
	cd ./tests/example-receiver && $(MAKE) clean	

.PHONY: compile-receiver
compile-receiver:
	cd ./tests/example-receiver && $(MAKE) build

.PHONY: doc
doc:
	cargo doc --no-deps 
	rm -rf ../snip1155-doc/docs
	cp -r ./target/doc ../snip1155-doc/docs

.PHONY: tarpaulin
tarpaulin:
	cargo tarpaulin \
		--exclude-files tests/example-receiver/src/contract.rs \
		--exclude-files tests/example-receiver/src/state.rs \
		--output-dir ./target/tarpaulin -o html
	wslview target/tarpaulin/tarpaulin-report.html
