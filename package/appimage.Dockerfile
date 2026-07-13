# Heavily inspired from
# https://github.com/13hannes11/gtk4-rs-docker/blob/main/appimage/Dockerfile

FROM fedora:36
ARG RUST_VERSION=stable
ENV RUST_VERSION=$RUST_VERSION

RUN sed -i \
        -e 's/^metalink=/#metalink=/' \
        -e 's/^#baseurl=http:\/\/download.example\/pub\/fedora\/linux/baseurl=https:\/\/dl.fedoraproject.org\/pub\/archive\/fedora\/linux/' \
        /etc/yum.repos.d/fedora.repo /etc/yum.repos.d/fedora-updates.repo \
    && dnf install -y --nogpgcheck gtk4-devel gcc libadwaita-devel openssl-devel curl wget file desktop-file-utils appstream squashfs-tools

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH=/root/.cargo/bin:$PATH
RUN rustup install ${RUST_VERSION}


ENV APPIMAGE_VERSION=continuous
ENV APPIMAGE_EXTRACT_AND_RUN=1

#RUN cargo install cargo-appimage
RUN cargo install --git https://github.com/PaulCombal/cargo-appimage.git

RUN wget https://github.com/AppImage/appimagetool/releases/download/$APPIMAGE_VERSION/appimagetool-x86_64.AppImage
RUN chmod +x appimagetool-x86_64.AppImage
RUN ./appimagetool-x86_64.AppImage --appimage-extract
RUN ln -nfs /squashfs-root/usr/bin/appimagetool /usr/bin/appimagetool

WORKDIR /mnt

CMD ["/bin/bash"]
