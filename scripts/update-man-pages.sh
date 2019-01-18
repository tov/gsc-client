#!/bin/sh

tar c man | tar xvC "$TOV_PUB/share"
chmod -R a+rX "$TOV_PUB/share/man"
