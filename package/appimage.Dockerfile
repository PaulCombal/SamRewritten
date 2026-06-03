# Inspired from https://github.com/13hannes11/gtk4-rs-docker/blob/main/appimage/Dockerfile

FROM fedora:40
ARG RUST_VERSION=stable
ENV RUST_VERSION=$RUST_VERSION

RUN dnf install gtk4-devel gcc libadwaita-devel openssl-devel wget file desktop-file-utils appstream squashfs-tools gettext -y

ENV APPIMAGE_VERSION=continuous
ENV APPIMAGE_EXTRACT_AND_RUN=1

RUN wget https://github.com/AppImage/appimagetool/releases/download/$APPIMAGE_VERSION/appimagetool-x86_64.AppImage \
    && chmod +x appimagetool-x86_64.AppImage \
    && ./appimagetool-x86_64.AppImage --appimage-extract \
    && ln -nfs /squashfs-root/usr/bin/appimagetool /usr/bin/appimagetool \
    && rm appimagetool-x86_64.AppImage

ARG USER_UID=1001
ARG USER_GID=1001
RUN groupadd -g ${USER_GID} builder \
    && useradd -m -u ${USER_UID} -g ${USER_GID} -s /bin/bash builder

COPY package/bundle-icons.sh /bundle-icons.sh
RUN chmod 755 /bundle-icons.sh

USER builder
ENV HOME=/home/builder
ENV PATH=/home/builder/.cargo/bin:$PATH

# Install Rust into the builder user's home (not /root).
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain ${RUST_VERSION} --profile minimal

#RUN cargo install cargo-appimage
RUN cargo install --git https://github.com/PaulCombal/cargo-appimage.git

WORKDIR /mnt

ENTRYPOINT ["/bundle-icons.sh"]
CMD ["/bin/bash"]
