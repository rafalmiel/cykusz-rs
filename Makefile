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
rust_shell := userspace/target/$(target_user)/debug/shell
rust_init := userspace/target/$(target_user)/debug/init
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

run: $(disk)
	#qemu-system-x86_64 -drive format=raw,file=$(iso) -serial stdio -no-reboot -m 512 -smp cpus=1  -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=ck_net0 -device e1000,netdev=ck_net0,id=ck_nic0
	#qemu-system-x86_64 -serial stdio -no-reboot -m 5811 -smp cpus=4 -netdev user,id=mynet0,net=192.168.1.0/24,dhcpstart=192.168.1.128,hostfwd=tcp::4444-:80 -device e1000e,netdev=mynet0,id=ck_nic0 -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -rtc base=utc,clock=host --enable-kvm
	#/home/ck/code/qemu/build/qemu-system-x86_64
	qemu-system-x86_64 \
        -cpu host \
        -d cpu \
        -serial stdio \
        -no-reboot \
        -m 5811 \
        -audio driver=pipewire,model=hda \
        -smp cpus=4 \
        -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=hn0 -device e1000,netdev=hn0,id=nic1 \
        -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 \
        -rtc base=utc,clock=host \
        -enable-kvm
	#qemu-system-x86_64 -serial stdio -no-reboot -m 5811 -smp cpus=4 -audio driver=pipewire,model=hda -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -rtc base=utc,clock=host --enable-kvm
	#qemu-system-x86_64 -serial stdio -no-reboot -m 512 -smp cpus=4 -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -rtc base=utc,clock=host

run_ata: $(disk)
	#qemu-system-x86_64 -drive format=raw,file=$(iso) -serial stdio -no-reboot -m 512 -smp cpus=4  -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=ck_net0 -device e1000,netdev=ck_net0,id=ck_nic0
	qemu-system-x86_64 -serial stdio -no-reboot -m 512 -smp cpus=4 -netdev user,id=mynet0,net=192.168.1.0/24,dhcpstart=192.168.1.128,hostfwd=tcp::4444-:80 -device e1000,netdev=mynet0,id=ck_nic0 -hda disk.img -rtc base=utc,clock=host

vbox: $(disk) vdi
	touch ./vbox_serial.log
	losetup -D
	losetup /dev/loop0 ./disk.img
	VBoxManage startvm cykusz  -E VBOX_GUI_DBG_AUTO_SHOW=false -E VBOX_GUI_DBG_ENABLED=false
	losetup -D
ifdef logs
	less +F --exit-follow-on-close ./vbox_serial.log
endif

vbox_debug: $(disk) vdi
	touch ./vbox_serial.log
	losetup -D
	losetup /dev/loop0 ./disk.img
	VBoxManage startvm cykusz  -E VBOX_GUI_DBG_AUTO_SHOW=true -E VBOX_GUI_DBG_ENABLED=true
	losetup -D
ifdef logs
	less +F --exit-follow-on-close ./vbox_serial.log
endif

debug: $(disk)
	#qemu-system-x86_64 -drive format=raw,file=$(iso) -serial stdio -no-reboot -s -S -smp cpus=4 -no-shutdown -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=ck_net0 -device e1000,netdev=ck_net0,id=ck_nic0
	#qemu-system-x86_64 -serial stdio -no-reboot -s -S -m 5811 -smp cpus=4 -no-shutdown -netdev user,id=mynet0,net=192.168.1.0/24,dhcpstart=192.168.1.128,hostfwd=tcp::4444-:80 -device e1000e,netdev=mynet0,id=ck_nic0 -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -monitor /dev/stdout
	qemu-system-x86_64 -serial stdio -no-reboot -s -S -m 5811 -smp cpus=4 -no-shutdown -netdev tap,helper=/usr/lib/qemu/qemu-bridge-helper,id=hn0 -device e1000,netdev=hn0,id=nic1 -drive format=raw,file=disk.img,if=none,id=test-img -device ich9-ahci,id=ahci -device ide-hd,drive=test-img,bus=ahci.0 -audio driver=pipewire,model=hda -monitor /dev/stdout

gdb:
	@rust-gdb "$(kernel)" -ex "target remote :1234"

kdbg:
	@kdbg -r localhost:1234

bochs: $(disk)
	@rm -f disk.img.lock
	bochs -f bochsrc.txt -q

$(disk): $(kernel) cargo_user $(cross_cpp)
ifdef dev
	CYKUSZ_LOGS=$(logs) disk-scripts/install_os.sh debug
else
	CYKUSZ_LOGS=$(logs) disk-scripts/install_os.sh release
endif

$(vdi): $(disk)
	disk-scripts/make_vmdk.sh
	disk-scripts/attach_vdi.sh

$(kernel): cargo_kernel $(rust_os) $(assembly_object_files) $(linker_script)
	ld -n --whole-archive --gc-sections -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_os)

usb: $(kernel)
	disk-scripts/install_usb.sh $(usb_dev)

vdi: $(vdi)

cargo_kernel:
ifdef dev
ifdef logs
	cd cykusz-rs && cargo build --verbose -F logs && cd ../
else
	cd cykusz-rs && cargo build --verbose && cd ../
endif
else
ifdef logs
	cd cykusz-rs && cargo build --release --verbose -F logs && cd ../
else
	cd cykusz-rs && cargo build --release --verbose && cd ../
endif
endif

cargo_user:
ifdef dev
	cd userspace && cargo build --verbose && cd ../
else
	cd userspace && cargo build --release --verbose && cd ../
endif

toolchain: $(cross_cpp)
	sysroot/build.sh check_build

fsck:
	disk-scripts/fsck_disk.sh

fsck_fix:
	disk-scripts/fsck_disk_fix.sh

$(cross_cpp): toolchain

# compile assembly files
build/arch/$(arch)/asm/%.o: cykusz-rs/src/arch/$(arch)/asm/%.asm
	mkdir -p $(shell dirname $@)
	nasm -felf64 $< -o $@
