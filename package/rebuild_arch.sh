# This is a dev script
# It is not possible to build in a bind-mounted volume, so we build outside the mount and copy the result

rm -rf /tmp/build
mkdir -p /tmp/build
cp /mnt/package/PKGBUILD /tmp/build
pushd /tmp/build
makepkg
popd
cp /tmp/build/*.zst /mnt/package