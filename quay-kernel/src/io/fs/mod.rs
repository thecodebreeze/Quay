use alloc::string::String;
use alloc::vec::Vec;

pub mod btrfs;

pub trait FileSystem {
    fn read_file(&self, inode: u64, buffer: &mut [u8]) -> Result<usize, &'static str>;
    fn list_dir(&self, inode: u64) -> Result<Vec<String>, &'static str>;
}
