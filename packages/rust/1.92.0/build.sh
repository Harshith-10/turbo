#!/bin/bash
set -e

INSTALL_DIR=$1
VERSION="1.92.0"

# Detect OS and Architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$OS" == "linux" ]; then
    if [ "$ARCH" == "x86_64" ]; then
        TARGET="x86_64-unknown-linux-gnu"
    elif [ "$ARCH" == "aarch64" ]; then
        TARGET="aarch64-unknown-linux-gnu"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
elif [ "$OS" == "darwin" ]; then
    if [ "$ARCH" == "x86_64" ]; then
        TARGET="x86_64-apple-darwin"
    elif [ "$ARCH" == "arm64" ]; then
        TARGET="aarch64-apple-darwin"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
else
    echo "Unsupported OS: $OS"
    exit 1
fi

TARBALL="rust-$VERSION-$TARGET.tar.xz"
URL="https://static.rust-lang.org/dist/$TARBALL"
DIRNAME="rust-$VERSION-$TARGET"

echo "Downloading Rust $VERSION ($TARGET) from $URL..."
curl --fail --retry 3 -L -O "$URL"

echo "Extracting..."
tar -xf "$TARBALL"

echo "Installing..."
cd "$DIRNAME"
./install.sh --prefix="$INSTALL_DIR" --disable-ldconfig

echo "Cleaning up..."
cd ..
rm -rf "$DIRNAME" "$TARBALL"

echo "Rust $VERSION installed successfully."
