FROM rust:latest

WORKDIR /mnt

RUN apt-get update
RUN apt-get install mingw-w64 libgtk-4-dev curl zstd -y
RUN curl -L "https://mirror.msys2.org/mingw/mingw64/mingw-w64-x86_64-gtk4-4.18.5-2-any.pkg.tar.zst" -o /tmp/gtk4.pkg.tar.zst
RUN curl -L "https://mirror.msys2.org/mingw/mingw64/mingw-w64-x86_64-pango-1.56.3-2-any.pkg.tar.zst" -o /tmp/pango.pkg.tar.zst
RUN curl -L "https://mirror.msys2.org/mingw/mingw64/mingw-w64-x86_64-glib2-2.84.2-1-any.pkg.tar.zst" -o /tmp/glib2.pkg.tar.zst
RUN curl -L "https://mirror.msys2.org/mingw/mingw64/mingw-w64-x86_64-gdk-pixbuf2-2.42.12-4-any.pkg.tar.zst" -o /tmp/gdkpx2.pkg.tar.zst
RUN curl -L "https://mirror.msys2.org/mingw/mingw64/mingw-w64-x86_64-harfbuzz-11.2.1-1-any.pkg.tar.zst" -o /tmp/harfbuzz.pkg.tar.zst
RUN curl -L "https://mirror.msys2.org/mingw/mingw64/mingw-w64-x86_64-cairo-1.18.4-2-any.pkg.tar.zst" -o /tmp/cairo.pkg.tar.zst
RUN curl -L "https://mirror.msys2.org/mingw/mingw64/mingw-w64-x86_64-graphene-1.10.8-2-any.pkg.tar.zst" -o /tmp/graphene.pkg.tar.zst
RUN tar -xf /tmp/gtk4.pkg.tar.zst -C /tmp
RUN tar -xf /tmp/pango.pkg.tar.zst -C /tmp
RUN tar -xf /tmp/glib2.pkg.tar.zst -C /tmp
RUN tar -xf /tmp/gdkpx2.pkg.tar.zst -C /tmp
RUN tar -xf /tmp/harfbuzz.pkg.tar.zst -C /tmp
RUN tar -xf /tmp/cairo.pkg.tar.zst -C /tmp
RUN tar -xf /tmp/graphene.pkg.tar.zst -C /tmp
RUN cp -r /tmp/mingw64/* /usr/x86_64-w64-mingw32/
RUN rm -rf /tmp/*

ENV PKG_CONFIG_SYSROOT_DIR=/usr/x86_64-w64-mingw32/

RUN rustup target add x86_64-pc-windows-gnu