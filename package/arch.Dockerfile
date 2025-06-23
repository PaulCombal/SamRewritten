FROM archlinux:latest

# ENV PKGEXT=".pkg.tar.zst" # Default Arch package extension

RUN pacman -Syu --noconfirm \
    base-devel \
    git \
    rust \
    gtk4 \
    libadwaita

# Create a non-root user for building packages
# It's good practice to avoid building as root for security and permission reasons.
RUN useradd -m -g users -G wheel builder
RUN echo "builder ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

USER builder
WORKDIR /mnt
CMD ["bash"]
