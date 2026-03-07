#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
#[allow(clippy::enum_variant_names)]
pub enum Order {
    Order4KiB = 0,
    Order8KiB = 1,
    Order16KiB = 2,
    Order32KiB = 3,
    Order64KiB = 4,
    Order128KiB = 5,
    Order256KiB = 6,
    Order512KiB = 7,
    Order1MiB = 8,
    Order2MiB = 9,
    Order4MiB = 10,
    Order8MiB = 11,
    Order16MiB = 12,
    Order32MiB = 13,
    Order64MiB = 14,
    Order128MiB = 15,
    Order256MiB = 16,
    Order512MiB = 17,
    Order1GiB = 18,
}
