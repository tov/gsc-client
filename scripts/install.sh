#!/bin/sh

set -x

umask 022

git_uri='https://github.com/tov/gsc-client.git'
install_source="--git ${git_uri} --branch release"
install_params=

while [ $# -gt 0 ]; do
    case $1 in
        (-l)
            install_source='--path .'
            shift
            ;;
        (*)
            echo >&2 "Didn’t understand ‘$1’"
            exit 1
            ;;
    esac
done

if [ -n "${PUB211-}" ]; then
    "$(dirname $0)/update-man-pages.sh"
else
    install_params=--all-features
fi

cargo install --force $install_source $install_params
