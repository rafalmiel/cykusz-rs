FROM ubuntu:noble

ARG uid=1000
ARG gid=1000
ARG user=ck
ARG workdir=/home/ck/code/cykusz-rs

RUN userdel -r ubuntu

RUN groupadd -g "$gid" $user
RUN useradd -u "$uid" -g "$gid" -m -d /home/$user $user

RUN apt-get update
RUN apt-get -y install curl nasm meson ninja-build bison flex libgmp3-dev libmpc-dev libmpfr-dev texinfo xorriso gettext gnulib autopoint gcc-12 g++-12 gperf linux-libc-dev python3.12 groff gengetopt cmake git rsync wget help2man

RUN update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-12 12
RUN update-alternatives --install /usr/bin/g++ g++ /usr/bin/g++-12 12

USER $user

RUN mkdir -p $workdir
WORKDIR $workdir/sysroot

RUN curl https://sh.rustup.rs -sSf | \
    sh -s -- -y

ENV PATH=/home/$user/.cargo/bin:$PATH

CMD ["bash"]
