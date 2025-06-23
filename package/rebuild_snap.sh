#!/bin/bash

# This is a dev script
# Launch this script from the project root directory
# So far the Snap version is not working in strict mode. The dev mode is used for screenshots.

rm *.snap
snap remove samrewritten
snapcraft
snap install --devmode --dangerous *.snap
#snap install --dangerous *.snap