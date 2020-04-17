#!/bin/sh

set -x

umask 022

"$(dirname $0)/update-man-pages.sh"

OPENSSL_STATIC=yes
export OPENSSL_STATIC
OPENSSL_INCLUDE_DIR=$TOV_PUB/include
export OPENSSL_INCLUDE_DIR
OPENSSL_LIB_DIR=$TOV_PUB/lib64
export OPENSSL_LIB_DIR

if [ "$1" = -l ]; then
    cargo install --verbose --force \
        --path .
else
    cargo install --verbose --force \
        --git https://github.com/tov/gsc-client.git
fi
