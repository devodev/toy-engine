use std::borrow::Cow;
use std::time::{Duration, Instant};

use log::{debug, error, info, trace, warn};

// inspired from: https://gitlab.com/imp/easytiming-rs/-/blob/master/src/lib.rs
#[derive(Debug)]
pub struct Timing<'a> {
    start: Instant,
    level: log::Level,
    msg: Cow<'a, str>,
}

impl<'a> Default for Timing<'a> {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            level: log::Level::Trace,
            msg: "TIME!".into(),
        }
    }
}

#[allow(unused)]
impl<'a> Timing<'a> {
    pub fn new<N>(msg: N) -> Self
    where
        N: Into<Cow<'a, str>>,
    {
        let mut t = Self::default();
        t.msg = msg.into();
        t
    }

    #[inline]
    fn elapsed(&self) -> Duration {
        Instant::now().duration_since(self.start)
    }

    #[inline]
    fn log(&self) {
        match self.level {
            log::Level::Error => error!("[{:?}] {}", self.elapsed(), self.msg),
            log::Level::Warn => warn!("[{:?}] {}", self.elapsed(), self.msg),
            log::Level::Info => info!("[{:?}] {}", self.elapsed(), self.msg),
            log::Level::Debug => debug!("[{:?}] {}", self.elapsed(), self.msg),
            log::Level::Trace => trace!("[{:?}] {}", self.elapsed(), self.msg),
        }
    }
}

impl<'a> Drop for Timing<'a> {
    fn drop(&mut self) {
        self.log()
    }
}

#[macro_export]
#[cfg(debug_assertions)]
macro_rules! TIME {
    () => {
        let _x = $crate::debug::Timing::default();
    };
    ($msg:expr) => {
        let _x = $crate::debug::Timing::new($msg);
    };
    ($($arg:expr),*) => {
        let _x = $crate::debug::Timing::new(format!($($arg),*));
    };
}
#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! TIME {
    () => {
        ()
    };
    ($msg: expr) => {
        ()
    };
    ($($arg:expr),*) => {
        ()
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    const MSG: &str = "timing";

    #[test]
    fn empty() {
        let t: Timing = Timing::default();
        assert_eq!(t.msg, "TIME!");
    }

    #[test]
    fn fromstr() {
        let t: Timing = Timing::new(MSG);
        assert_eq!(t.msg, MSG);
    }

    #[test]
    fn fromstring() {
        let t: Timing = Timing::new(String::from(MSG));
        assert_eq!(t.msg, MSG);
    }

    #[test]
    fn fromborrowed() {
        let t: Timing = Timing::new(Cow::Borrowed(MSG));
        assert_eq!(t.msg, MSG);
    }

    #[test]
    fn fromowned() {
        let t: Timing = Timing::new(Cow::Owned(String::from(MSG)));
        assert_eq!(t.msg, MSG);
    }
}
