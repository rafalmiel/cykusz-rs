arch ?= x86_64
kernel := build/kernel-$(arch).bin
iso := build/os-$(arch).iso

linker_script := src/arch/$(arch)/asm/linker.ld
grub_cfg := src/arch/$(arch)/asm/grub.cfg
assembly_source_files := $(wildcard src/arch/$(arch)/asm/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/asm/%.asm, \
		build/arch/$(arch)/asm/%.o, $(assembly_source_files))

target ?= $(arch)-unknown-none-gnu
rust_os := target/$(target)/release/libcykusz_rs.a

.PHONY: all clean run iso

all: $(kernel)

userspace/$(target)/release/program: userspace/src/main.rs
	cd userspace && RUST_TARGET_PATH=`pwd` RUSTFLAGS="-Z no-landing-pads -C link-arg=-no-pie -C link-arg=-static"  xargo build --release --target ../$(target) --verbose

clean:
	cargo clean
	find build -name *.o | xargs rm | true

purge: clean
	rm -rf build
	rm -rf target

run: $(iso)
	qemu-system-x86_64 -drive format=raw,file=$(iso) -no-reboot -m 512 -smp cpus=4 -no-shutdown
debug: $(iso)
	qemu-system-x86_64 -drive format=raw,file=$(iso) -no-reboot -s -S -smp cpus=4 -no-shutdown
gdb:
	#@rust-os-gdb/bin/rust-gdb "build/kernel-x86_64.bin" -ex "target remote :1234"
	@rust-gdb "build/kernel-x86_64.bin" -ex "target remote :1234"
kdbg:
	@kdbg -r localhost:1234 "build/kernel-x86_64.bin"
bochs: $(iso)
	bochs -f bochsrc.txt -q

iso: $(iso)

$(iso): $(kernel) $(grub_cfg) userspace/$(target)/release/program
	mkdir -p build/isofiles/boot/grub
	cp $(kernel) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	cp userspace/$(target)/release/program build/isofiles/boot/program
	grub-mkrescue -d /usr/lib/grub/i386-pc/ -o $(iso) build/isofiles 2> /dev/null

$(kernel): cargo $(rust_os) $(assembly_object_files) $(linker_script)
	ld -n --whole-archive --gc-sections -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_os)

cargo:
	RUST_TARGET_PATH=`pwd` RUSTFLAGS="-Z no-landing-pads"  xargo build --release --target $(target) --verbose

# compile assembly files
build/arch/$(arch)/asm/%.o: src/arch/$(arch)/asm/%.asm
	mkdir -p $(shell dirname $@)
	nasm -felf64 $< -o $@
