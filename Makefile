arch ?= x86_64
iso := build/os-$(arch).iso
disk := disk.img
vdi := disk.vdi

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
cross_gcc := sysroot/cross/bin/x86_64-cykusz-g++
cross_strip := sysroot/cross/bin/x86_64-cykusz-strip
cross_hello := sysroot/build/hello

.PHONY: all clean run ata bochs iso toolchain fsck

all: $(kernel) $(user)

clean:
	cargo clean
	find build -name *.o | xargs rm | true

purge: clean
	rm -rf build
	rm -rf target

run: $(iso) $(disk)
	#qemu-system-x86_64 -drive format=raw,file=$(iso) -serial stdio -no-reboot -m 512 -smp cpus=1  -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=ck_net0 -device e1000,netdev=ck_net0,id=ck_nic0
	qemu-system-x86_64 -serial stdio -no-reboot -m 512 -smp cpus=4 -netdev user,id=mynet0,net=192.168.1.0/24,dhcpstart=192.168.1.128,hostfwd=tcp::4444-:80 -device e1000,netdev=mynet0,id=ck_nic0 -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -rtc base=utc,clock=host

run_ata: $(iso) $(disk)
	#qemu-system-x86_64 -drive format=raw,file=$(iso) -serial stdio -no-reboot -m 512 -smp cpus=4  -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=ck_net0 -device e1000,netdev=ck_net0,id=ck_nic0
	qemu-system-x86_64 -serial stdio -no-reboot -m 512 -smp cpus=1 -netdev user,id=mynet0,net=192.168.1.0/24,dhcpstart=192.168.1.128,hostfwd=tcp::4444-:80 -device e1000,netdev=mynet0,id=ck_nic0 -hda disk.img -rtc base=utc,clock=host

vbox: $(iso) $(vdi)
	VBoxManage startvm cykusz # -E VBOX_GUI_DBG_AUTO_SHOW=true -E VBOX_GUI_DBG_ENABLED=true

debug: $(iso) $(disk)
	#qemu-system-x86_64 -drive format=raw,file=$(iso) -serial stdio -no-reboot -s -S -smp cpus=4 -no-shutdown -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=ck_net0 -device e1000,netdev=ck_net0,id=ck_nic0
	qemu-system-x86_64 -serial stdio -no-reboot -s -S -m 512 -smp cpus=1 -no-shutdown -netdev user,id=mynet0,net=192.168.1.0/24,dhcpstart=192.168.1.128,hostfwd=tcp::4444-:80 -device e1000,netdev=mynet0,id=ck_nic0 -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -monitor /dev/stdout

gdb:
	@rust-gdb "$(kernel)" -ex "target remote :1234"

kdbg:
	@kdbg -r localhost:1234 "$(kernel)"

bochs: $(iso) $(disk)
	@rm -f disk.img.lock
	/home/ck/code/bochs-svn/src/bochs-svn/bochs -f bochsrc.txt -q

iso: $(iso)

$(iso): $(kernel) $(grub_cfg) $(user)
	mkdir -p build/isofiles/boot/grub
	cp $(kernel) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	cp $(user) build/isofiles/boot/program
	grub-mkrescue -d /usr/lib/grub/i386-pc/ -o $(iso) build/isofiles 2> /dev/null

$(disk): $(iso) hello
	sudo disk_scripts/install_os.sh

$(vdi): $(disk)
	disk_scripts/make_vdi.sh
	disk_scripts/attach_vdi.sh

$(kernel): cargo $(rust_os) $(assembly_object_files) $(linker_script)
	ld -n --whole-archive --gc-sections -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_os)

cargo:
ifdef dev
	RUST_TARGET_PATH=`pwd` xargo build --workspace --target $(target) --verbose
else
	RUST_TARGET_PATH=`pwd` xargo build --workspace --release --target $(target) --verbose
endif

toolchain: $(cross_gcc)
	sysroot/build.sh check_build

fsck:
	sudo disk_scripts/fsck_disk.sh

$(cross_gcc): toolchain

hello: $(cross_gcc) sysroot/hello.cpp
	$(cross_gcc) sysroot/hello.cpp -o $(cross_hello)
	$(cross_strip) $(cross_hello)

# compile assembly files
build/arch/$(arch)/asm/%.o: cykusz-rs/src/arch/$(arch)/asm/%.asm
	mkdir -p $(shell dirname $@)
	nasm -felf64 $< -o $@
