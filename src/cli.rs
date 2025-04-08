//! This module implements the CLI interface.
//!
//! The main external constraint here is that we need to be compatible
//! to the vfio-user [Backend Program
//! Conventions](https://github.com/nutanix/libvfio-user/blob/master/docs/vfio-user.rst#backend-program-conventions).
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    long_about = None
)]
pub struct Cli {
    /// Enable verbose logging. Can be specified multiple times to
    /// increase verbosity.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// The path where to create a listening Unix domain socket.
    ///
    /// This is the path where Cloud Hypervisor will connect to usbvfiod.
    #[arg(short, long)]
    pub socket_path: PathBuf,
}
