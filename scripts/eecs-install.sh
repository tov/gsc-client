#!/bin/sh

set -x

OPENSSL_STATIC=yes
export OPENSSL_STATIC
OPENSSL_INCLUDE_DIR=$TOV_PUB/include
export OPENSSL_INCLUDE_DIR
OPENSSL_LIB_DIR=$TOV_PUB/lib64
export OPENSSL_LIB_DIR

cargo install --verbose --force --path .
install -m 755 man/gsc.1 $TOV_PUB/share/man/man1/
