#!/bin/bash
set -e

INSTALL_DIR=$1
VERSION="25.0.1"

# Detect OS and Architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$OS" == "linux" ]; then
    if [ "$ARCH" == "x86_64" ]; then
        PLATFORM="linux-x64"
    elif [ "$ARCH" == "aarch64" ]; then
        PLATFORM="linux-aarch64"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
elif [ "$OS" == "darwin" ]; then
    if [ "$ARCH" == "x86_64" ]; then
        PLATFORM="macos-x64"
    elif [ "$ARCH" == "arm64" ]; then
        PLATFORM="macos-aarch64"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
else
    echo "Unsupported OS: $OS"
    exit 1
fi

TARBALL="jdk-25_${PLATFORM}_bin.tar.gz"
URL="https://download.oracle.com/java/25/latest/$TARBALL"

echo "Downloading Java $VERSION ($PLATFORM) from $URL..."
curl -L -O "$URL"

echo "Extracting..."
mkdir -p "$INSTALL_DIR"
tar -xf "$TARBALL" -C "$INSTALL_DIR" --strip-components=1

echo "Cleaning up..."
rm "$TARBALL"

echo "Java $VERSION installed successfully."
