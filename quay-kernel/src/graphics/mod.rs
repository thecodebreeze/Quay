use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::convert::Infallible;
use core::ptr::copy_nonoverlapping;
use embedded_graphics::Pixel;
use embedded_graphics::geometry::Size;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use limine::framebuffer::Framebuffer;
use log::{error, info, warn};

/// Double-buffered wrapper around the hardware framebuffer.
pub struct DoubleBuffer {
    back_buffer: Vec<u32>,
    front_buffer: *mut u32,
    width: u64,
    height: u64,
    pitch: u64,
}

impl DoubleBuffer {
    /// Wrap the hardware framebuffer in a double-buffered wrapper.
    pub fn new(framebuffer: &Framebuffer) -> Self {
        let width = framebuffer.width();
        let height = framebuffer.height();
        let pitch = framebuffer.pitch();
        let bpp = framebuffer.bpp();

        // We only support 32-bit framebuffers (4 bytes per pixel).
        assert_eq!(bpp, 32, "Only 32-bit framebuffers are supported.");

        // Calculate the number of pixels in the framebuffer and allocate the back buffer.
        let pixels_count = (pitch / 4) * height;
        let back_buffer = vec![0; pixels_count as usize];

        Self {
            back_buffer,
            front_buffer: framebuffer.addr() as *mut u32,
            width,
            height,
            pitch,
        }
    }

    /// Paint the contents of the back buffer to the screen.
    pub fn flush(&self) {
        unsafe {
            copy_nonoverlapping(
                self.back_buffer.as_ptr(),
                self.front_buffer,
                self.back_buffer.len(),
            )
        }
    }
}

impl OriginDimensions for DoubleBuffer {
    fn size(&self) -> Size {
        Size::new(self.width as u32, self.height as u32)
    }
}

impl DrawTarget for DoubleBuffer {
    type Color = Rgb888;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            // Drop pixels that are off-screen.
            if coord.x >= 0
                && coord.x < self.width as i32
                && coord.y >= 0
                && coord.y < self.height as i32
            {
                let index = ((coord.y as u64 * (self.pitch / 4)) + coord.x as u64) as usize;
                let raw_color =
                    ((color.r() as u32) << 16) | ((color.g() as u32) << 8) | (color.b() as u32);
                self.back_buffer[index] = raw_color;
            }
        }

        Ok(())
    }
}

/// A simple wrapper to allow `edid_rs` to read from a raw memory slice in `no_std`.
struct SliceReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> edid_rs::Read for SliceReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Option<usize> {
        if self.pos >= self.data.len() {
            return Some(0); // EOF
        }
        let remain = self.data.len() - self.pos;
        let read_len = core::cmp::min(remain, buf.len());

        buf[..read_len].copy_from_slice(&self.data[self.pos..self.pos + read_len]);
        self.pos += read_len;

        Some(read_len)
    }
}

pub fn log_edid_info(fb: &Framebuffer) {
    let (edid_ptr, edid_size) = match fb.edid() {
        Some(edid) => (edid.as_ptr(), edid.len()),
        None => {
            warn!("No EDID provided by the firmware.");
            return;
        }
    };
    if edid_ptr.is_null() || edid_size == 0 {
        warn!("No EDID provided by the firmware.");
        return;
    }

    let edid_slice = unsafe { core::slice::from_raw_parts(edid_ptr, edid_size) };
    let mut reader = SliceReader {
        data: edid_slice,
        pos: 0,
    };

    match edid_rs::parse(&mut reader) {
        Ok(edid) => {
            // Extract Monitor Name and Serial Number from the descriptors.
            let mut monitor_name = String::from("Generic Monitor");
            let mut serial_string = String::from("N/A");

            for desc in &edid.descriptors.0 {
                match desc {
                    edid_rs::MonitorDescriptor::MonitorName(name) => monitor_name = name.clone(),
                    edid_rs::MonitorDescriptor::SerialNumber(serial) => {
                        serial_string = serial.clone()
                    }
                    _ => {}
                }
            }

            // Format the Manufacturer ID (stored as 3 compressed characters).
            let mfg = edid.product.manufacturer_id;
            let mfg_str = format!("{}{}{}", mfg.0, mfg.1, mfg.2);

            // Print the Header.
            info!("========================================");
            info!("          DISPLAY EDID INFO             ");
            info!("========================================");
            info!(" Monitor Name  : {}", monitor_name);
            info!(
                " Manufacturer  : {} (Code: {})",
                mfg_str, edid.product.product_code
            );
            info!(" Serial Number : {}", serial_string);
            info!(
                " EDID Version  : {}.{}",
                edid.version.version, edid.version.revision
            );
            info!(
                " Manuf. Date   : Week {}, Year {}",
                edid.product.manufacture_date.week, edid.product.manufacture_date.year
            );

            // Physical Dimensions.
            if let Some(size) = &edid.display.max_size {
                info!(
                    " Physical Size : {} cm x {} cm",
                    size.width as u32, size.height as u32
                );
            }

            // Calculate Preferred Timing (Resolution & Refresh Rate).
            if let Some(timing) = edid.timings.detailed_timings.first() {
                let w = timing.active.0;
                let h = timing.active.1;

                // Calculate total pixels including blanking (porches and sync).
                let h_total = w + timing.front_porch.0 + timing.sync_length.0 + timing.back_porch.0;
                let v_total = h + timing.front_porch.1 + timing.sync_length.1 + timing.back_porch.1;

                // Refresh Rate (Hz) = Pixel Clock / (H_Total * V_Total).
                let refresh_rate = timing.pixel_clock / (h_total as u32 * v_total as u32);

                info!(" Preferred Res : {}x{} @ {}Hz", w, h, refresh_rate);
                info!(" Pixel Clock   : {} MHz", timing.pixel_clock / 1_000_000);
            }

            info!("========================================");
        }
        Err(error) => {
            error!("Failed to parse EDID: {}", error);
        }
    }
}
