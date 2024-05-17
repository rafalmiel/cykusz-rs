arch ?= x86_64
iso := build/os-$(arch).iso
disk := disk.img
vdi := disk.vdi

linker_script := cykusz-rs/src/arch/$(arch)/asm/linker.ld
grub_cfg := cykusz-rs/src/arch/$(arch)/asm/grub.cfg
assembly_source_files := $(wildcard cykusz-rs/src/arch/$(arch)/asm/*.asm)
assembly_object_files := $(patsubst cykusz-rs/src/arch/$(arch)/asm/%.asm, \
		build/arch/$(arch)/asm/%.o, $(assembly_source_files))

target ?= $(arch)-cykusz_os
target_user ?= $(arch)-unknown-cykusz
ifdef dev
rust_os := target/$(target)/debug/libcykusz_rs.a
rust_shell := userspace/target/$(target)/debug/shell
rust_init := userspace/target/$(target)/debug/init
kernel := build/kernel-$(arch)-g.bin
else
rust_os := target/$(target)/release/libcykusz_rs.a
rust_shell := userspace/target/$(target_user)/release/shell
rust_init := userspace/target/$(target_user)/release/init
kernel := build/kernel-$(arch).bin
endif
cross_c := sysroot/cross/bin/x86_64-cykusz-gcc
cross_cpp := sysroot/cross/bin/x86_64-cykusz-g++
cross_clang := sysroot/cross/bin/clang --sysroot sysroot/cykusz/  -target x86_64-cykusz
cross_clangpp := sysroot/cross/bin/clang++ --sysroot sysroot/cykusz/  -target x86_64-cykusz

usb_dev := /dev/sdb1

.PHONY: all clean run ata bochs iso toolchain fsck

all: cargo_kernel cargo_user

clean:
	cargo clean
	find build -name *.o | xargs rm | true

purge: clean
	rm -rf build
	rm -rf target

run: $(iso) $(disk)
	#qemu-system-x86_64 -drive format=raw,file=$(iso) -serial stdio -no-reboot -m 512 -smp cpus=1  -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=ck_net0 -device e1000,netdev=ck_net0,id=ck_nic0
	#qemu-system-x86_64 -serial stdio -no-reboot -m 5811 -smp cpus=4 -netdev user,id=mynet0,net=192.168.1.0/24,dhcpstart=192.168.1.128,hostfwd=tcp::4444-:80 -device e1000e,netdev=mynet0,id=ck_nic0 -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -rtc base=utc,clock=host --enable-kvm
	qemu-system-x86_64 -serial stdio -no-reboot -m 5811 -smp cpus=4 -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=hn0 -device e1000,netdev=hn0,id=nic1 -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -rtc base=utc,clock=host --enable-kvm
	#qemu-system-x86_64 -serial stdio -no-reboot -m 512 -smp cpus=4 -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -rtc base=utc,clock=host

run_ata: $(iso) $(disk)
	#qemu-system-x86_64 -drive format=raw,file=$(iso) -serial stdio -no-reboot -m 512 -smp cpus=4  -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=ck_net0 -device e1000,netdev=ck_net0,id=ck_nic0
	qemu-system-x86_64 -serial stdio -no-reboot -m 512 -smp cpus=4 -netdev user,id=mynet0,net=192.168.1.0/24,dhcpstart=192.168.1.128,hostfwd=tcp::4444-:80 -device e1000,netdev=mynet0,id=ck_nic0 -hda disk.img -rtc base=utc,clock=host

vbox: $(iso) $(vdi)
	VBoxManage startvm cykusz  -E VBOX_GUI_DBG_AUTO_SHOW=true -E VBOX_GUI_DBG_ENABLED=true

debug: $(iso) $(disk)
	#qemu-system-x86_64 -drive format=raw,file=$(iso) -serial stdio -no-reboot -s -S -smp cpus=4 -no-shutdown -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=ck_net0 -device e1000,netdev=ck_net0,id=ck_nic0
	#qemu-system-x86_64 -serial stdio -no-reboot -s -S -m 5811 -smp cpus=4 -no-shutdown -netdev user,id=mynet0,net=192.168.1.0/24,dhcpstart=192.168.1.128,hostfwd=tcp::4444-:80 -device e1000e,netdev=mynet0,id=ck_nic0 -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -monitor /dev/stdout
	qemu-system-x86_64 -serial stdio -no-reboot -s -S -m 5811 -smp cpus=4 -no-shutdown -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=hn0 -device e1000,netdev=hn0,id=nic1 -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -monitor /dev/stdout

gdb:
	@rust-gdb "$(kernel)" -ex "target remote :1234"

kdbg:
	@kdbg -r localhost:1234

bochs: $(iso) $(disk)
	@rm -f disk.img.lock
	bochs -f bochsrc.txt -q

iso: $(iso)

$(iso): $(kernel) $(grub_cfg) $(rust_shell)
	mkdir -p build/isofiles/boot/grub
	cp $(kernel) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	cp $(rust_shell) build/isofiles/boot/program
	grub-mkrescue -d /usr/lib/grub/i386-pc/ -o $(iso) build/isofiles 2> /dev/null

$(disk): $(kernel) cargo_user $(cross_cpp)
	#echo fake install_os
	sudo disk-scripts/install_os.sh

$(vdi): $(disk)
	disk-scripts/make_vdi.sh
	disk-scripts/attach_vdi.sh

$(kernel): cargo_kernel $(rust_os) $(assembly_object_files) $(linker_script)
	ld -n --whole-archive --gc-sections -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_os)

usb: $(kernel)
	sudo disk-scripts/install_usb.sh $(usb_dev)

cargo_kernel:
ifdef dev
	cd cykusz-rs && cargo build  --verbose && cd ../
else
	cd cykusz-rs && cargo build --release --verbose && cd ../
endif

cargo_user:
	sysroot/build.sh cargo_userspace

toolchain: $(cross_cpp)
	sysroot/build.sh check_build

fsck:
	sudo disk-scripts/fsck_disk.sh

$(cross_cpp): toolchain

# compile assembly files
build/arch/$(arch)/asm/%.o: cykusz-rs/src/arch/$(arch)/asm/%.asm
	mkdir -p $(shell dirname $@)
	nasm -felf64 $< -o $@
