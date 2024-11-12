#!/bin/bash

wget https://dl-cdn.alpinelinux.org/alpine/v3.20/releases/x86_64/alpine-minirootfs-3.20.3-x86_64.tar.gz
mkdir -p alpine
cd alpine
tar -xf ../alpine-minirootfs-3.20.3-x86_64.tar.gz
cd ..
rm alpine-minirootfs-3.20.3-x86_64.tar.gz