FROM ubuntu:jammy

RUN apt-get update
RUN apt-get -y install nasm meson ninja-build bison flex libgmp3-dev libmpc-dev libmpfr-dev texinfo xorriso autotools-dev automake autoconf autoconf-archive gettext pkgconf autopoint gcc-12 g++-12 gperf linux-libc-dev python3.11 groff gengetopt cmake git rsync wget help2man
RUN update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-12 12
RUN update-alternatives --install /usr/bin/g++ g++ /usr/bin/g++-12 12

RUN mkdir -p /home/ck/cykusz-rs
WORKDIR /home/ck/cykusz-rs/sysroot

CMD ["bash"]
