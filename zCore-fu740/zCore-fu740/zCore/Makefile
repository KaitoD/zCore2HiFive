################ Arguments ################

ARCH ?= x86_64
PLATFORM ?= qemu
MODE ?= debug
LOG ?= warn
LINUX ?=
LIBOS ?=
GRAPHIC ?=
HYPERVISOR ?=
SMP ?= 1
ACCEL ?=
V ?=
USER ?=
ZBI ?= bringup
CMDLINE ?=
ARGS ?= /bin/busybox

OBJDUMP ?= rust-objdump --print-imm-hex --x86-asm-syntax=intel
OBJCOPY ?= rust-objcopy --binary-architecture=$(ARCH)

################ Internal variables ################

qemu := qemu-system-$(ARCH)
build_path := ../target/$(ARCH)/$(MODE)
kernel_elf := $(build_path)/zcore
kernel_img := $(build_path)/zcore.bin
kernel_board := $(build_path)/zcore-fu740
esp := $(build_path)/esp
ovmf := ../rboot/OVMF.fd
qemu_disk := $(build_path)/disk.qcow2

ifeq ($(shell uname), Darwin)
  sed := sed -i ""
else
  sed := sed -i
endif

ifeq ($(LINUX), 1)
  user_img := $(ARCH).img
else ifeq ($(USER), 1)
  user_img := ../zircon-user/target/zcore-user.zbi
else
  user_img := ../prebuilt/zircon/x64/$(ZBI).zbi
endif

################ Export environments ###################

export ARCH
export PLATFORM
export LOG
export USER_IMG=$(realpath $(user_img))

################ Cargo features ################

ifeq ($(LINUX), 1)
  features := linux
else
  features := zircon
endif

ifeq ($(PLATFORM), libos)
  LIBOS := 1
endif

ifeq ($(LIBOS), 1)
  ifneq ($(ARCH), $(shell uname -m))
    $(error "ARCH" must be "$(shell uname -m)" for libos mode)
  endif
  features += libos
else
  ifeq ($(ARCH), x86_64)
    features += init_ram_disk
  else ifeq ($(ARCH), riscv64)
    ifeq ($(PLATFORM), d1)
      features += board_d1 link_user_img
		else ifeq ($(PLATFORM), fu740)
			features += board_fu740 link_user_img
    else
      features += board_qemu
    endif
  endif
endif

ifeq ($(GRAPHIC), on)
  features += graphic
else ifeq ($(MAKECMDGOALS), vbox)
  features += graphic
endif

ifeq ($(HYPERVISOR), 1)
  features += hypervisor
  ACCEL := 1
endif

################ Cargo build args ################

build_args := --features "$(features)"

ifneq ($(LIBOS), 1) # no_std
  build_args += --target $(ARCH).json -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem
endif

ifeq ($(MODE), release)
  build_args += --release
endif

ifeq ($(V), 1)
  build_args += --verbose
endif

################ QEMU options ################

qemu_opts := -smp $(SMP)

ifeq ($(ARCH), x86_64)
  baremetal-test-qemu_opts := \
		-machine q35 \
		-cpu Haswell,+smap,-check,-fsgsbase \
		-m 4G \
		-serial mon:stdio \
		-drive format=raw,if=pflash,readonly=on,file=$(ovmf) \
		-drive format=raw,file=fat:rw:$(esp) \
		-device ich9-ahci,id=ahci \
		-device isa-debug-exit,iobase=0xf4,iosize=0x04 \
		-nic none
  qemu_opts += $(baremetal-test-qemu_opts) \
		-drive format=qcow2,id=userdisk,if=none,file=$(qemu_disk) \
		-device ide-hd,bus=ahci.0,drive=userdisk
else ifeq ($(ARCH), riscv64)
  qemu_opts += \
		-machine virt \
		-bios default \
		-kernel $(kernel_img) \
		-no-reboot \
		-no-shutdown \
		-serial mon:stdio \
		-drive format=qcow2,id=userdisk,file=$(qemu_disk) \
		-device virtio-blk-device,drive=userdisk
endif

ifeq ($(GRAPHIC), on)
  ifeq ($(ARCH), x86_64)
    qemu_opts += -vga virtio # disable std VGA for zircon mode to avoid incorrect graphic rendering
  else ifeq ($(ARCH), riscv64)
    qemu_opts += \
		-device virtio-gpu-device \
		-device virtio-keyboard-device \
		-device virtio-mouse-device
  endif
else
  qemu_opts += -display none -nographic
  baremetal-test-qemu_opts += -display none -nographic
endif

ifeq ($(ACCEL), 1)
  ifeq ($(shell uname), Darwin)
    qemu_opts += -accel hvf
  else
    qemu_opts += -accel kvm -cpu host,migratable=no,+invtsc
  endif
endif

################ Make targets ################

.PHONY: all
all: build

.PHONY: build run test debug
ifeq ($(LIBOS), 1)
build: kernel
run:
	cargo run $(build_args) -- $(ARGS)
test:
	cargo test $(build_args)
debug: build
	gdb --args ../target/$(MODE)/zcore $(ARGS)
else
build: $(kernel_img)
run: build justrun
debug: build debugrun
endif

.PHONY: justrun
justrun: $(qemu_disk)
	$(qemu) $(qemu_opts)

.PHONY: debugrun
debugrun: $(qemu_disk)
	$(qemu) $(qemu_opts) -s -S &
	@sleep 1
	gdb $(kernel_elf) -tui -x gdbinit

.PHONY: kernel
kernel:
	@echo Building zCore kernel
	cargo build $(build_args)

.PHONY: disasm
disasm:
	$(OBJDUMP) -d $(kernel_elf) | less

.PHONY: header
header:
	$(OBJDUMP) -x $(kernel_elf) | less

.PHONY: clippy
clippy:
	cargo clippy $(build_args)

fu740:
	gzip -9 -cvf $(build_path)/zcore.bin > $(build_path)/zcore.bin.gz
	mkimage -A riscv -O linux -C gzip -T kernel -a 80200000 -e 80200000 -n "zCore-fu740" -d $(build_path)/zcore.bin.gz $(build_path)/zcore-fu740
	@echo 'Build zcore fu740 image done'

.PHONY: clean
clean:
	cargo clean

.PHONY: bootloader
bootloader:
ifeq ($(ARCH), x86_64)
	@cd ../rboot && make build
endif

$(kernel_img): kernel bootloader
ifeq ($(ARCH), x86_64)
  ifeq ($(USER), 1)
	make -C ../zircon-user
  endif
	mkdir -p $(esp)/EFI/zCore $(esp)/EFI/Boot
	cp ../rboot/target/x86_64-unknown-uefi/release/rboot.efi $(esp)/EFI/Boot/BootX64.efi
	cp rboot.conf $(esp)/EFI/Boot/rboot.conf
	cp $(kernel_elf) $(esp)/EFI/zCore/zcore.elf
	cp $(user_img) $(esp)/EFI/zCore/
	$(sed) "s/fuchsia.zbi/$(notdir $(user_img))/" $(esp)/EFI/Boot/rboot.conf
  ifneq ($(CMDLINE),)
	$(sed) "s#cmdline=.*#cmdline=$(CMDLINE)#" $(esp)/EFI/Boot/rboot.conf
  else
	$(sed) "s/LOG=warn/LOG=$(LOG)/" $(esp)/EFI/Boot/rboot.conf
  endif
else ifeq ($(ARCH), riscv64)
	$(OBJCOPY) $(kernel_elf) --strip-all -O binary $@
endif

.PHONY: image
image:
# for macOS only
	hdiutil create -fs fat32 -ov -volname EFI -format UDTO -srcfolder $(esp) $(build_path)/zcore.cdr
	qemu-img convert -f raw $(build_path)/zcore.cdr -O qcow2 $(build_path)/zcore.qcow2

################ Tests ################

.PHONY: baremetal-qemu-disk
baremetal-qemu-disk:
	@qemu-img create -f qcow2 $(build_path)/disk.qcow2 100M

.PHONY: baremetal-test
baremetal-test:
	cp rboot.conf $(esp)/EFI/Boot/rboot.conf
	timeout --foreground 8s  $(qemu) $(baremetal-test-qemu_opts)

.PHONY: baremetal-test-rv64
baremetal-test-rv64: build $(qemu_disk)
	timeout --foreground 8s $(qemu) $(qemu_opts) -append ROOTPROC=$(ROOTPROC)

################ Deprecated ################

VMDISK := $(build_path)/boot.vdi

.PHONY: vbox
vbox: build
ifneq "$(VMDISK)" "$(wildcard $(VMDISK))"
	vboxmanage createvm --name zCoreVM --basefolder $(build_path) --register
	cp ../prebuilt/zircon/empty.vdi $(VMDISK)
	vboxmanage storagectl zCoreVM --name DiskCtrlr --controller IntelAhci --add sata
	vboxmanage storageattach zCoreVM --storagectl DiskCtrlr --port 0 --type hdd --medium $(VMDISK)
	vboxmanage modifyvm zCoreVM --memory 1024 --firmware efi
	tar -cvf $(build_path)/esp.tar -C $(build_path)/esp EFI
	sudo LIBGUESTFS_DEBUG=1 guestfish -a $(VMDISK) -m /dev/sda1 tar-in $(build_path)/esp.tar / : quit
endif
	# depencency: libguestfs-tools
	tar -cvf $(build_path)/esp.tar -C $(build_path)/esp EFI
	sudo guestfish -a $(VMDISK) -m /dev/sda1 tar-in $(build_path)/esp.tar / : quit
	# sudo LIBGUESTFS_DEBUG=1 guestfish -a $(VMDISK) -m /dev/sda1 tar-in $(build_path)/esp.tar / : quit
	# -a $(VMDISK) $(build_path)/esp.tar /
	rm $(build_path)/esp.tar
	vboxmanage startvm zCoreVM

$(qemu_disk):
ifeq ($(ARCH), riscv64)
# FIXME: no longer need to create QCOW2 when use initrd for RISC-V
	@echo Generating riscv64 sfsimg
	@qemu-img convert -f raw riscv64.img -O qcow2 $@
	@qemu-img resize $@ +5M
else
	@qemu-img create -f qcow2 $@ 100M
endif
