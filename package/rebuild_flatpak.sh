#!/bin/bash

# This is a dev script
# Launch this script from the package directory

flatpak-builder --force-clean build-dir ./flatpak.yml
flatpak-builder --run build-dir flatpak.yml samrewritten