#!/bin/sh

set -x

"$(dirname $0)/update-man-pages.sh"

OPENSSL_STATIC=yes
export OPENSSL_STATIC
OPENSSL_INCLUDE_DIR=$TOV_PUB/include
export OPENSSL_INCLUDE_DIR
OPENSSL_LIB_DIR=$TOV_PUB/lib64
export OPENSSL_LIB_DIR

