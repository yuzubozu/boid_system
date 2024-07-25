# boid system

boid system by bevy

# build as wasm

'''
rustup target install wasm32-unknown-unknown
cargo install wasm-server-runner
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner
cargo build --release --target wasm32-unknown-unknown 
wasm-bindgen --target web --out-dir ./wasm ./target/wasm32-unknown-unknown/release/boid_bird.wasm
'''