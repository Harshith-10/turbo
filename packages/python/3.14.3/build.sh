#!/bin/bash
set -e

INSTALL_DIR=$1
VERSION="3.14.3"
BUILD_DATE="20260211"
PLATFORM="x86_64-unknown-linux-gnu"
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
