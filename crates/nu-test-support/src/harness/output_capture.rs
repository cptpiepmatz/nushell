use std::{cell::RefCell, sync::OnceLock};

pub static NO_CAPTURE: OnceLock<bool> = OnceLock::new();
pub static SHOW_OUTPUT: OnceLock<bool> = OnceLock::new();

thread_local! {
    pub static OUTPUT: RefCell<Vec<Output>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone)]
pub enum Output {
    Stdout(String),
    Stderr(String),
}

#[macro_export]
macro_rules! println {
    ($($args:tt)*) => {'block: {
        if $crate::harness::output_capture::NO_CAPTURE.get().cloned().unwrap_or(false) {
            ::std::println!($($args)*);
            break 'block;
        }

        $crate::harness::output_capture::OUTPUT.with_borrow_mut(|output| {
            let buf = ::std::format!($($args)*) + "\n";
            output.push($crate::harness::output_capture::Output::Stdout(buf));
        });
    }};
}

#[macro_export]
macro_rules! print {
    ($($args:tt)*) => {'block: {
        if $crate::harness::output_capture::NO_CAPTURE.get().cloned().unwrap_or(false) {
            ::std::print!($($args)*);
            break 'block;
        }

        $crate::harness::output_capture::OUTPUT.with_borrow_mut(|output| {
            let buf = ::std::format!($($args)*);
            output.push($crate::harness::output_capture::Output::Stdout(buf));
        });
    }};
}

#[macro_export]
macro_rules! eprintln {
    ($($args:tt)*) => {'block: {
        if $crate::harness::output_capture::NO_CAPTURE.get().cloned().unwrap_or(false) {
            ::std::eprintln!($($args)*);
            break 'block;
        }

        $crate::harness::output_capture::OUTPUT.with_borrow_mut(|output| {
            let buf = ::std::format!($($args)*) + "\n";
            output.push($crate::harness::output_capture::Output::Stderr(buf));
        });
    }};
}

#[macro_export]
macro_rules! eprint {
    ($($args:tt)*) => {'block: {
        if $crate::harness::output_capture::NO_CAPTURE.get().cloned().unwrap_or(false) {
            ::std::eprint!($($args)*);
            break 'block;
        }

        $crate::harness::output_capture::OUTPUT.with_borrow_mut(|output| {
            let buf = ::std::format!($($args)*);
            output.push($crate::harness::output_capture::Output::Stderr(buf));
        });
    }};
}
