FROM ubuntu:17.10

MAINTAINER Casey Primozic <me@ameo.link>

RUN apt-get update

# Add required dependencies
RUN apt-get install curl wget file build-essential libboost-all-dev postgresql postgresql-contrib -y

# Install the Rust nightly toolchain
# Adapted from https://hub.docker.com/r/mackeyja92/rustup/~/dockerfile/
RUN curl https://sh.rustup.rs -s > /home/install.sh && \
    chmod +x /home/install.sh && \
    sh /home/install.sh -y --verbose --default-toolchain nightly

ENV PATH "/root/.cargo/bin:$PATH"

# Install Node.JS v8.4.0
WORKDIR /home
RUN curl -s -o /tmp/nodejs.xz "https://nodejs.org/dist/v8.4.0/node-v8.4.0-linux-x64.tar.xz"
WORKDIR /home/node-v8.4.0-linux-x64
ENV PATH "/home/node-v8.4.0-linux-x64/bin:$PATH"

# Copy in source code and scripts
COPY ./ /app

# Build the platform
RUN make release

# Expose ports for the MM's WebSocket connection
EXPOSE 7037

# Start the platform
ENTRYPOINT "/app/run.sh"
