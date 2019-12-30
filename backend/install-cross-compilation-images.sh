#!/bin/sh
docker build -t contrasleuth-docker-support/cross-utils:aarch64-linux-android ./contrasleuth-docker-support/aarch64-linux-android
docker build -t contrasleuth-docker-support/cross-utils:arm-linux-androideabi ./contrasleuth-docker-support/arm-linux-androideabi
docker build -t contrasleuth-docker-support/cross-utils:armv7-linux-androideabi ./contrasleuth-docker-support/armv7-linux-androideabi
docker build -t contrasleuth-docker-support/cross-utils:i686-linux-android ./contrasleuth-docker-support/i686-linux-android
docker build -t contrasleuth-docker-support/cross-utils:x86_64-linux-android ./contrasleuth-docker-support/x86_64-linux-android
