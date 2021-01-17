#!/bin/sh

tar c man | tar xvC "$PUB211/share"
chmod -R a+rX "$PUB211/share/man"
