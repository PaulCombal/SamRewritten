#!/bin/bash

# This is a dev script
# Launch this script from the project root directory
# Classic confinement is for production.
# Strict with dev mode is used for screenshots.

rm *.snap
snap remove samrewritten
snapcraft
#snap install --devmode --dangerous *.snap
#snap install --dangerous *.snap
#snap connect samrewritten:access-steam-folder
snap install *.snap --classic --dangerous