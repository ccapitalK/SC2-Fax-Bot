#!/bin/bash

set -euxo pipefail

cargo build --release --target x86_64-unknown-linux-musl
mkdir -p .package
cp "target/x86_64-unknown-linux-musl/release/SC2-Fax-Bot" .package/FaxBot
pushd .package
zip FaxBot.zip FaxBot
mv FaxBot.zip ..
popd
rm -r .package
