#!/usr/bin/env bash
# Rebuild a frozen (deployed) contract and compare the sha256 with the on-chain code hash.
#
#   legacy/rebuild.sh locker-v1 | locker-v2 | locker3-lock3 | locker3-lock4 | locker3-lock_4 |
#                     token-raw-v1 | token-raw-v2 | token-tax-v1 | token-tax-v2 | token-tax-v3
#
# The deployed wasm embeds the crate's workspace-relative source path (panic locations,
# e.g. `contracts/locker/src/lib.rs`) and, for the lockers, the package name and version in
# the NEAR ABI and NEP-330 metadata. So each legacy crate is copied into a scratch workspace
# at its ORIGINAL path and package name before it is built; the crates under legacy/ carry
# distinct package names only so the audit workspace resolves.
#
# locker-v1 was built inside the sourcescan docker image (paths /home/near/.cargo and
# /rustc/<commit> are baked into the wasm), so it is rebuilt in that image; docker is needed.
# lock3, lock4 and lock_4 were built with `cargo near build non-reproducible-wasm --no-doc` (no
# rustdoc text in the embedded ABI), so they are rebuilt that way.
# token-raw-v1 (the code $NEARLY runs) was built with rustc 1.96.0, wasm MVP features only and
# no wasm-opt, so it is rebuilt that way (needs `rustup toolchain install 1.96.0`).
# Everything else builds on the host with the pinned toolchain (rust-toolchain.toml),
# cargo-near 0.22.0 and binaryen wasm-opt 132.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
what="${1:?which contract}"
case "$what" in
  locker-v1)    crate=contracts/locker;    pkg=nearpad-locker;    kind=locker; docker=1
                expect=fe4a58f7bb97dd858d290a01adab8b9afad2f5783c1ac1d12fe603fb394558de ;;
  locker-v2)    crate=contracts/locker;    pkg=nearpad-locker;    kind=locker; docker=0
                expect=ee2a2d48c807f67bf8af8fdbc29aceffc20bc3218d597326cfc034f6a8868c79 ;;
  locker3-lock3)  crate=contracts/locker3; pkg=nearpad-locker3; kind=locker-nodoc; docker=0
                expect=70f2a58e5a3a68b9011776cc47a13b0cbc8701ce0807fcde1aebe9b6dfba8eea ;;
  locker3-lock4)  crate=contracts/locker3; pkg=nearpad-locker3; kind=locker-nodoc; docker=0
                expect=845d2ccc539c3c85e46000f03e21994fcd03a18888b93db76d37c7133acfd30b ;;
  locker3-lock_4) crate=contracts/locker3; pkg=nearpad-locker3; kind=locker-nodoc; docker=0
                expect=97058d1c88659e60fc06487b6d36364db4b48c234b3ea552723acb6d46e65ba4 ;;
  token-raw-v1) crate=contracts/token-raw; pkg=nearpad-token-raw; kind=token-mvp; docker=0
                expect=47d7973b0a3bf97a5008f03c7caf15d02fc05ac0c34c1878bdf7ebabf6857fc9 ;;
  token-raw-v2) crate=contracts/token-raw; pkg=nearpad-token-raw; kind=token;  docker=0
                expect=95ec4ec33ef96f387164dbb894481f38b6b140ad2aa4059309576208e52cad17 ;;
  token-tax-v1) crate=contracts/token-tax; pkg=nearpad-token-tax; kind=token;  docker=0
                expect=003b1b46bfe37276642e126f3095265a10d5d2c81619bd2de1e7523e80dc2625 ;;
  token-tax-v2) crate=contracts/token-tax; pkg=nearpad-token-tax; kind=token;  docker=0
                expect=08134395464cd03f468b760e02cea160f9835649221bbb520127e524bc55160e ;;
  token-tax-v3) crate=contracts/token-tax; pkg=nearpad-token-tax; kind=token;  docker=0
                expect=aca1a7f83e219034ea96c0675bf3dff418b871d5a0f4213738a199aadd5f2f11 ;;
  *) echo "unknown: $what" >&2; exit 2 ;;
esac

W="${REBUILD_DIR:-$here/target/rebuild-$what}"
rm -rf "$W"; mkdir -p "$W/$crate"
cp -R "$here/legacy/$what/src" "$W/$crate/src"
sed "s/^name = \"nearpad-[a-z0-9-]*\"/name = \"$pkg\"/" "$here/legacy/$what/Cargo.toml" > "$W/$crate/Cargo.toml"
cp "$here/Cargo.lock" "$here/rust-toolchain.toml" "$W/"
python3 - "$here/Cargo.toml" "$W/Cargo.toml" "$crate" <<'EOF'
import re,sys
s=open(sys.argv[1]).read()
s=re.sub(r'members = \[[^\]]*\]', 'members = ["%s"]' % sys.argv[3], s, flags=re.S)
open(sys.argv[2],'w').write(s)
EOF

if [ "$kind" = token-mvp ]; then
  n="${pkg//-/_}"
  (cd "$W" && RUSTFLAGS="-C target-cpu=mvp -C target-feature=-bulk-memory,-sign-ext,-reference-types,-multivalue,-nontrapping-fptoint" \
    cargo +1.96.0 build --release --target wasm32-unknown-unknown -p "$pkg")
  out="$W/target/wasm32-unknown-unknown/release/$n.wasm"
elif [ "$kind" = token ]; then
  n="${pkg//-/_}"
  (cd "$W" && RUSTFLAGS="-C target-cpu=mvp" cargo build --release --target wasm32-unknown-unknown -p "$pkg")
  wasm-opt -Oz --enable-bulk-memory --enable-bulk-memory-opt --enable-sign-ext --enable-mutable-globals \
    "$W/target/wasm32-unknown-unknown/release/$n.wasm" -o "$W/$n.wasm"
  out="$W/$n.wasm"
elif [ "$kind" = locker-nodoc ]; then
  (cd "$W/$crate" && cargo near build non-reproducible-wasm --no-doc)
  out="$W/target/near/${pkg//-/_}/${pkg//-/_}.wasm"
elif [ "$docker" = 1 ]; then
  img="sourcescan/cargo-near:0.22.0-rust-1.97.1@sha256:7467038bdddc86484b73b416eeadce926ff59013e128e53dec5a19e1cb4b2234"
  rev=97fd423024a2fd0b15978292b45106bc133a34ac
  docker run --rm --volume "$W:/home/near/code" --user 0:0 --workdir "/home/near/code/$crate" \
    --env "NEP330_LINK=https://github.com/sam3dsol/Nearlytrade/tree/$rev" \
    --env "NEP330_BUILD_INFO_BUILD_ENVIRONMENT=$img" \
    --env 'NEP330_BUILD_INFO_BUILD_COMMAND=["cargo","near","build","non-reproducible-wasm","--locked"]' \
    --env "NEP330_BUILD_INFO_CONTRACT_PATH=$crate" \
    --env "NEP330_BUILD_INFO_SOURCE_CODE_SNAPSHOT=git+https://github.com/sam3dsol/Nearlytrade?rev=$rev" \
    --env "NEP330_BUILD_INFO_OUTPUT_WASM_PATH=/home/near/code/target/near/${pkg//-/_}/${pkg//-/_}.wasm" \
    "$img" cargo near build non-reproducible-wasm --locked
  out="$W/target/near/${pkg//-/_}/${pkg//-/_}.wasm"
else
  (cd "$W/$crate" && cargo near build non-reproducible-wasm)
  out="$W/target/near/${pkg//-/_}/${pkg//-/_}.wasm"
fi

got="$(shasum -a 256 "$out" | cut -d' ' -f1)"
echo "built:    $out"
echo "sha256:   $got"
echo "expected: $expect"
if [ "$got" = "$expect" ]; then echo "MATCH"; else echo "MISMATCH"; exit 1; fi
