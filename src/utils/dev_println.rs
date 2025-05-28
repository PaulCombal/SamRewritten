#[macro_export]
macro_rules! dev_println {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! dev_print {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            print!($($arg)*);
        }
    };
}