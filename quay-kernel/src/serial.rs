use lazy_static::lazy_static;
use log::{Level, Metadata, Record};
use spin::Mutex;
use uart_16550::SerialPort;

lazy_static! {
    // We wrap the serial port in a sping lock mutex so that if multiple parts of the kernel try to
    // print at the exact same time, they don't corrupt each other's data.
    pub static ref SERIAL1: Mutex<SerialPort> = {
        // 0x3F8 is the standard I/O port for the first serial interface (COM1).
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

/// Serial Logger global instance.
static LOGGER: SerialLogger = SerialLogger;

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1
        .lock()
        .write_fmt(args)
        .expect("Printing to serial failed!");
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::serial::_print(format_args!($($arg)*)));
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

/// Struct to act as a frontend for the `log` crate.
struct SerialLogger;

impl log::Log for SerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let file_name = record.file().unwrap_or("unknown");
            let line_number = record.line().unwrap_or(0);

            let timestamp = unsafe { core::arch::x86_64::_rdtsc() };

            crate::println!(
                "[{timestamp:>14}] [{level:>5}] {file}:{line} -> {args}",
                timestamp = timestamp,
                level = record.level(),
                file = file_name,
                line = line_number,
                args = record.args()
            );
        }
    }

    fn flush(&self) {}
}

/// Initializes the global logger so the `log` macros know where to send text.
pub fn init_logger() {
    log::set_logger(&LOGGER)
        .map(|_| log::set_max_level(log::LevelFilter::Trace))
        .ok();
}
