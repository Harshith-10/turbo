#!/bin/bash
set -e

INSTALL_DIR=$1
VERSION="1.92.0"
TARBALL="rust-$VERSION-x86_64-unknown-linux-gnu.tar.xz"
URL="https://static.rust-lang.org/dist/$TARBALL"

echo "Downloading Rust $VERSION from $URL..."
curl --fail --retry 3 -L -O "$URL"

echo "Extracting..."
tar -xf "$TARBALL"

echo "Installing..."
cd "rust-$VERSION-x86_64-unknown-linux-gnu"
./install.sh --prefix="$INSTALL_DIR" --disable-ldconfig

echo "Cleaning up..."
cd ..
rm -rf "rust-$VERSION-x86_64-unknown-linux-gnu" "$TARBALL"

echo "Rust $VERSION installed successfully."
