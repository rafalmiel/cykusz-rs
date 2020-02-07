arch ?= x86_64
iso := build/os-$(arch).iso

linker_script := cykusz-rs/src/arch/$(arch)/asm/linker.ld
grub_cfg := cykusz-rs/src/arch/$(arch)/asm/grub.cfg
assembly_source_files := $(wildcard cykusz-rs/src/arch/$(arch)/asm/*.asm)
assembly_object_files := $(patsubst cykusz-rs/src/arch/$(arch)/asm/%.asm, \
		build/arch/$(arch)/asm/%.o, $(assembly_source_files))

target ?= $(arch)-unknown-none-gnu
ifdef dev
rust_os := target/$(target)/debug/libcykusz_rs.a
user := target/$(target)/debug/program
kernel := build/kernel-$(arch)-g.bin
else
rust_os := target/$(target)/release/libcykusz_rs.a
user := target/$(target)/release/program
kernel := build/kernel-$(arch).bin
endif

.PHONY: all clean run iso

all: $(kernel) $(user)

clean:
	cargo clean
	find build -name *.o | xargs rm | true

purge: clean
	rm -rf build
	rm -rf target

run: $(iso)
	qemu-system-x86_64 -drive format=raw,file=$(iso) -no-reboot -m 512 -smp cpus=1 -no-shutdown

debug: $(iso)
	qemu-system-x86_64 -drive format=raw,file=$(iso) -no-reboot -s -S -smp cpus=1 -no-shutdown

gdb:
	@rust-gdb "$(kernel)" -ex "target remote :1234"

kdbg:
	@kdbg -r localhost:1234 "$(kernel)"

bochs: $(iso)
	bochs -f bochsrc.txt -q

iso: $(iso)

$(iso): $(kernel) $(grub_cfg) $(user)
	mkdir -p build/isofiles/boot/grub
	cp $(kernel) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	cp $(user) build/isofiles/boot/program
	grub-mkrescue -d /usr/lib/grub/i386-pc/ -o $(iso) build/isofiles 2> /dev/null

$(kernel): cargo $(rust_os) $(assembly_object_files) $(linker_script)
	ld -n --whole-archive --gc-sections -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_os)

cargo:
ifdef dev
	RUST_TARGET_PATH=`pwd` RUSTFLAGS="-Z no-landing-pads"  xargo build --workspace --target $(target) --verbose
else
	RUST_TARGET_PATH=`pwd` RUSTFLAGS="-Z no-landing-pads"  xargo build --workspace --release --target $(target) --verbose
endif

# compile assembly files
build/arch/$(arch)/asm/%.o: cykusz-rs/src/arch/$(arch)/asm/%.asm
	mkdir -p $(shell dirname $@)
	nasm -felf64 $< -o $@
