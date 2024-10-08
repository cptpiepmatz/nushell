# Git: git version 2.30.2
# /etc/os-release: Alpine Linux v3.16
# Kernel: Linux ca3abedc4fb1 5.17.15-76051715-generic #202206141358~1655919116~22.04~1db9e34 SMP PREEMPT Wed Jun 22 19 x86_64 Linux
# Build cmd: docker build --no-cache . -t nushell-latest
# Other tags: nushell/alpine-nu:latest, nushell
FROM alpine

LABEL maintainer=nushell

ARG TARGETARCH

RUN set -eux; \
    if [ "${TARGETARCH}" = "amd64" ] || [ "${TARGETARCH}" = "x86_64" ]; then \
        echo "Downloading x86_64 binary for ${TARGETARCH}..."; \
        arch_path="x86_64"; \
    elif [ "${TARGETARCH}" = "arm64" ] || [ "${TARGETARCH}" = "aarch64"]; then \
        echo "Downloading aarch64 binary for ${TARGETARCH}..."; \
        arch_path="aarch64"; \
    else \
        arch_path=""; \
    fi; \
    echo '/usr/bin/nu' >> /etc/shells \
    && adduser -D -s /usr/bin/nu nushell \
    && mkdir -p /home/nushell/.config/nushell/ \
    && cd /tmp \
    && wget -qO - https://api.github.com/repos/nushell/nushell/releases/latest \
    | grep browser_download_url \
    | grep "${arch_path}.*.musl.tar.gz" \
    | cut -f4 -d '"' \
    | xargs -I{} wget {} \
    && tar -xzf nu* \
    && cd nu*-musl \
    && mv nu* /usr/bin \
    && chmod +x /usr/bin/nu \
    && chown -R nushell:nushell /home/nushell/.config/nushell \
    # Reset Nushell config to default
    && su -c 'config reset -w' nushell \
    && ls /usr/bin/nu_plugin* \
    | xargs -I{} su -c 'plugin add {}' nushell \
    && rm -rf /tmp/*

USER nushell

WORKDIR /home/nushell

ENTRYPOINT ["nu"]
