#!/bin/bash
set -e

INSTALL_DIR=$1
VERSION="3.13.11"
TARBALL="Python-$VERSION.tar.xz"
URL="https://www.python.org/ftp/python/$VERSION/$TARBALL"

echo "Downloading Python $VERSION from $URL..."
curl -L -O "$URL"

echo "Extracting..."
tar -xf "$TARBALL"

echo "Configuring..."
cd "Python-$VERSION"
# Use --enable-optimizations if you have time, omitted here for speed in dev env
./configure --prefix="$INSTALL_DIR" --disable-test-modules

echo "Compiling (this may take a while)..."
make -j$(nproc)

echo "Installing..."
make install

echo "Cleaning up..."
cd ..
rm -rf "Python-$VERSION" "$TARBALL"

echo "Python $VERSION installed successfully."
