# ⚓ Quay

**Quay** is a hobbyist kernel and operating system written from scratch in **Rust**. It aims to be a modern, safe, and efficient kernel targeting the `x86_64` architecture.

## 🚀 Current Status

Quay is in early development. Currently, it boots using the **Limine** bootloader and initializes several core subsystems:

- **Architecture**: x86_64 (64-bit).
- **Bootloader**: [Limine](https://limine-bootloader.org/) (Revision 5).
- **Memory Management**:
  - **Physical memory manager**: Frame Allocator for allocating 4KiB physical frames.
  - **Virtual memory manager**: Page Table Mapper for configuring virtual memory maps.
  - **Kernel heap allocator**: Global heap managed using the [Talc](https://github.com/creativcoder/talc) allocator (128MiB default heap size).
- **Interrupts & System Tables**:
  - **Global Descriptor Table (GDT)** and **Task State Segment (TSS)** with an Interrupt Stack Table (IST) for safe double fault handling.
  - **Interrupt Descriptor Table (IDT)** for handling hardware and software interrupts, including:
    - **CPU Exceptions**: Page Fault, Double Fault, General Protection Fault, Breakpoint, and Divide-by-Zero handling.
    - **Hardware Interrupts**: Timer and Keyboard interrupts.
  - **Advanced Programmable Interrupt Controller (APIC)**: Initialization of both Local APIC and I/O APIC via ACPI for modern interrupt handling.
- **Hardware Abstraction**:
  - **ACPI Support**: Using the `acpi` crate to find and configure system devices.
  - **Serial Logging**: Real-time logging over `COM1` serial port using `uart_16550`.
  - **UEFI Framebuffer**: Graphics support for basic visual output (clearing the screen).
  - **Keyboard Support**: Basic scancode reading from the PS/2 controller.

## 🛠 Getting Started

### Prerequisites

To build and run Quay, you'll need the following tools installed on your system:

- **Rust Nightly**: Required for several experimental features.
- **Just**: A command-line runner used for building and running.
- **QEMU**: Hardware emulator for running the kernel.
- **mtools**: For FAT32 disk image manipulation (`mformat`, `mcopy`).
- **parted**: For partitioning the disk image.
- **OVMF**: For UEFI support in QEMU.

### Build and Run

To compile the kernel and launch it in QEMU, simply run:

```bash
just run
```

This command will:
1. Compile the kernel for the `x86_64-quay-kernel` target.
2. Prepare a staging area with the kernel and Limine EFI files.
3. Create a GPT-partitioned disk image.
4. Format a FAT32 ESP partition and inject the required files.
5. Launch QEMU with KVM acceleration, VirtIO support, and UEFI firmware.

## 📁 Project Structure

- `quay-kernel/src/main.rs`: The kernel entry point and initialization sequence.
- `quay-kernel/src/x86/`: x86_64 specific implementations:
  - `gdt/`: Global Descriptor Table and Task State Segment.
  - `interrupt/`: IDT and APIC configuration.
  - `acpi/`: ACPI table parsing and handling.
- `quay-kernel/src/memory/`: Memory management (physical, virtual, and heap).
- `quay-kernel/src/serial.rs`: Serial logging implementation.
- `quay-kernel/etc/limine/`: Limine bootloader configuration and binary files.

---

*Happy hacking!* 🌊
