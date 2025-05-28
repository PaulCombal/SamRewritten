#!/bin/bash
# Launch this script from the project root directory

rm *.snap
snap remove samrewritten
snapcraft
snap install --devmode --dangerous *.snap