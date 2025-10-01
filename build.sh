echo "Building loader" && cargo build --target x86_64-pc-lilium-kernel --profile dev --manifest-path loader/Cargo.toml --target-dir target/ || exit 1
echo "Building Modules" && cargo build --target x86_64-pc-lilium-kernel --profile dev --manifest-path modules/Cargo.toml --workspace --target-dir target/
cp -v target/x86_64-pc-lilium-loader/debug/liblilium_loader.so lilium-loader.so

mkdir -p ovmf
if [ ! -f ovmf/ovmf-code-x86_64.fd ]; then
    curl -Lo ovmf/ovmf-code-x86_64.fd https://github.com/osdev0/edk2-ovmf-nightly/releases/latest/download/ovmf-code-x86_64.fd
fi
if [ ! -f ovmf/ovmf-vars-x86_64.fd ]; then
    curl -Lo ovmf/ovmf-vars-x86_64.fd https://github.com/osdev0/edk2-ovmf-nightly/releases/latest/download/ovmf-vars-x86_64.fd
fi

if [ ! -f limine/Makefile ]; then
    git submodule update --init || exit 1
fi

if [ ! -f limine/limine ]; then
    make -C limine || exit 1
fi

rm -rf iso_root

mkdir -p iso_root/boot/modules
cp -v lilium-loader.so iso_root/boot/lilium-loader.so
cp -v target/x86_64-pc-lilium-loader/debug/libhello_world.so iso_root/boot/modules/hello_world.so

mkdir -p iso_root/boot/limine
cp -v limine.conf iso_root/boot/limine
mkdir -p iso_root/EFI/BOOT
cp -v limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin iso_root/boot/limine
cp -v limine/BOOTX64.EFI iso_root/EFI/BOOT
cp -v limine/BOOTIA32.EFI iso_root/EFI/BOOT
xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot boot/limine/limine-uefi-cd.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o os-for-fun.iso

limine/limine bios-install os-for-fun.iso
# rm -rf iso_root
