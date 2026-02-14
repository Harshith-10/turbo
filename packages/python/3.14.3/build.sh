#!/bin/bash
set -e

INSTALL_DIR=$1
VERSION="3.14.3"
BUILD_DATE="20260211"

# Detect OS and Architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$OS" == "linux" ]; then
    if [ "$ARCH" == "x86_64" ]; then
        PLATFORM="x86_64-unknown-linux-gnu"
    elif [ "$ARCH" == "aarch64" ]; then
        PLATFORM="aarch64-unknown-linux-gnu"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
elif [ "$OS" == "darwin" ]; then
    if [ "$ARCH" == "x86_64" ]; then
        PLATFORM="x86_64-apple-darwin"
    elif [ "$ARCH" == "arm64" ]; then
        PLATFORM="aarch64-apple-darwin"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
else
    echo "Unsupported OS: $OS"
    exit 1
fi

FILENAME="cpython-$VERSION+$BUILD_DATE-$PLATFORM-install_only.tar.gz"
URL="https://github.com/astral-sh/python-build-standalone/releases/download/$BUILD_DATE/$FILENAME"

echo "Downloading Python $VERSION (Standalone Build) from $URL..."
curl -L -O "$URL"

echo "Extracting..."
mkdir -p "$INSTALL_DIR"
# The archive contains a single directory named 'python' at the root.
# We strip it to install directly into INSTALL_DIR.
tar -xf "$FILENAME" -C "$INSTALL_DIR" --strip-components=1

echo "Cleaning up..."
rm "$FILENAME"

echo "Python $VERSION installed successfully."
