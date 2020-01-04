use std::error::Error;
use std::convert::TryFrom;

use std::os::unix::io::{AsRawFd, RawFd};

use nix::poll::PollFlags;
use nix::sys::signal::{SigSet, Signal};
use nix::sys::signalfd::{SfdFlags, SignalFd};

use crate::context::Context;
use crate::pollable::{PollRunResult, Pollable};

pub struct Signals {
    listener: SignalFd,
}

impl Signals {
    pub fn new() -> Result<Signals, Box<dyn Error>> {
        let mut mask = SigSet::empty();
        mask.add(Signal::SIGALRM);
        mask.add(Signal::SIGTERM);
        mask.add(Signal::SIGCHLD);
        mask.thread_block()?;

        let listener = SignalFd::with_flags(&mask, SfdFlags::SFD_NONBLOCK | SfdFlags::SFD_CLOEXEC)?;

        Ok(Signals { listener })
    }
}

impl Pollable for Signals {
    fn fd(&self) -> RawFd {
        self.listener.as_raw_fd()
    }

    fn poll_flags(&self) -> PollFlags {
        PollFlags::POLLIN
    }

    fn run(&mut self, ctx: &mut Context) -> Result<PollRunResult, Box<dyn Error>> {
        loop {
            match self.listener.read_signal() {
                Ok(Some(sig)) => match Signal::try_from(sig.ssi_signo as i32)? {
                    Signal::SIGALRM => ctx.alarm()?,
                    Signal::SIGCHLD => ctx.check_children()?,
                    Signal::SIGTERM => ctx.terminate()?,
                    _ => (),
                },
                Ok(None) => break Ok(PollRunResult::Uneventful),
                Err(err) => break Err(err.into()),
            }
        }
    }
}
