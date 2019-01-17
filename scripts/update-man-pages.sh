#!/bin/sh

for i in man/*; do
    section=$(echo "$i" | sed 's/.*[.]//')
    install -dm 755 "$i" "$TOV_PUB/share/man/man$section/"
done
