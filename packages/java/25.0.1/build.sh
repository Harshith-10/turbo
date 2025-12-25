#!/bin/bash
set -e

INSTALL_DIR=$1
VERSION="25.0.1" # User requested 25.0.1, using latest link as requested
URL="https://download.oracle.com/java/25/latest/jdk-25_linux-x64_bin.tar.gz"
TARBALL="jdk-25_linux-x64_bin.tar.gz"

echo "Downloading Java $VERSION from $URL..."
curl -L -O "$URL"

echo "Extracting..."
mkdir -p "$INSTALL_DIR"
# Strip the top-level directory (usually jdk-25.x.x)
tar -xf "$TARBALL" -C "$INSTALL_DIR" --strip-components=1

echo "Cleaning up..."
rm "$TARBALL"

echo "Java $VERSION installed successfully."
