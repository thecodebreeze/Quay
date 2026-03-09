MODE := "debug"
PROFILE := if MODE == "debug" { "dev" } else { "release" }

KERNEL_BIN := "quay-kernel/target/x86_64-quay-kernel/" + MODE + "/quay-kernel"
BUILD_DIR := "build"
SYSROOT := "build/sysroot"
QUAY_IMG := "build/quay.img"
QUAY_CONFIG := "quay-kernel/etc/quay/quay.ron"
ROOTFS_IMG := "build/rootfs.btrfs"

# Default recipe
run: build-all image qemu

# Compile all sub-projects.
compile:
    @echo "==> Compiling Kernel..."
    cd quay-kernel && just MODE={{MODE}}

# Gather everything into a dynamic sysroot.
build-all: compile
    @echo "==> Preparing sysroot..."
    rm -rf {{SYSROOT}}

    mkdir -p {{SYSROOT}}/efi/EFI/BOOT
    mkdir -p {{SYSROOT}}/efi/quay/v0.0.1
    cp {{KERNEL_BIN}} {{SYSROOT}}/efi/quay/v0.0.1/quay-kernel
    cp {{QUAY_CONFIG}} {{SYSROOT}}/efi/quay/v0.0.1/quay.ron
    cp quay-kernel/etc/limine/limine.conf {{SYSROOT}}/efi/EFI/BOOT/
    cp quay-kernel/etc/limine/BOOTX64.EFI {{SYSROOT}}/efi/EFI/BOOT/

    mkdir -p {{SYSROOT}}/rootfs/bin
    mkdir -p {{SYSROOT}}/rootfs/etc

    echo "quay" > {{SYSROOT}}/rootfs/etc/hostname

# Create the Disk Image with two partitions.
image: build-all
    @echo "==> Building BTRFS Root Partition"

    truncate -s 64M {{ROOTFS_IMG}}
    mkfs.btrfs --rootdir {{SYSROOT}}/rootfs -f {{ROOTFS_IMG}} > /dev/null

    @echo "==> Building Main Disk Image..."
    dd if=/dev/zero of={{QUAY_IMG}} bs=1M count=98 status=none

    @echo "==> Partitioning image (GPT)..."
    parted -s {{QUAY_IMG}} mklabel gpt
    parted -s {{QUAY_IMG}} mkpart ESP fat32 1MiB 33MiB
    parted -s {{QUAY_IMG}} set 1 esp on
    parted -s {{QUAY_IMG}} mkpart ROOT btrfs 33MiB 97MiB

    @echo "==> Setting deterministic PARTUUID for ROOT partition..."
    sgdisk --partition-guid=1:dad4987f-f229-4c53-ae9d-7530f52d7597 {{QUAY_IMG}}
    sgdisk --partition-guid=2:a7093d6a-8eac-42ba-9dd8-1fbaf458187f {{QUAY_IMG}}

    @echo "==> Injecting EFI Partition (mtools)..."
    mformat -i {{QUAY_IMG}}@@1048576 -F -v "QUAY_EFI" ::
    mcopy -i {{QUAY_IMG}}@@1048576 -s {{SYSROOT}}/efi/* ::/

    @echo "==> Injecting BTRFS Partition (dd)..."
    dd if={{ROOTFS_IMG}} of={{QUAY_IMG}} bs=1M seek=33 conv=notrunc status=none
    rm {{ROOTFS_IMG}}

# Launch the emulator.
qemu:
    @echo "Launching QEMU with KVM and full VirtIO stack..."
    qemu-system-x86_64 \
        -M q35 \
        -accel kvm \
        -cpu host,+invtsc \
        -m 16G \
        -bios /usr/share/ovmf/OVMF.fd \
        -display gtk,gl=on \
        -device virtio-vga-gl \
        -drive id=disk0,format=raw,file={{QUAY_IMG}},if=none \
        -device virtio-blk-pci,drive=disk0 \
        -netdev user,id=net0 \
        -device virtio-net-pci,netdev=net0 \
        -device virtio-rng-pci \
        -device virtio-keyboard-pci \
        -device virtio-mouse-pci \
        -serial stdio