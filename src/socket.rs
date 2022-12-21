use nix::{
    cmsg_space,
    sys::{
        socket::{ControlMessageOwned, MsgFlags, SockaddrIn},
        time::TimeSpec,
    },
};
use tokio::io::unix::{AsyncFd, TryIoError};

use std::{
    io::{IoSlice, IoSliceMut},
    marker::PhantomData,
    os::unix::prelude::{AsRawFd, RawFd},
};

pub struct AsyncSocket<'a, T: 'a> {
    inner: AsyncFd<RawFd>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> AsyncSocket<'a, T> {
    pub fn new(fd: RawFd) -> tokio::io::Result<Self> {
        Ok(Self {
            // inner: AsyncFd::new(fd)?,
            inner: AsyncFd::new(fd)?,
            phantom: PhantomData,
        })
    }

    pub async fn write_to(
        &'a self,
        buffer: &'a [IoSlice<'_>; 1],
        socket_address: &SockaddrIn,
    ) -> Result<usize, TryIoError> {
        let mut guard = self.inner.writable().await.unwrap();
        let flags = MsgFlags::empty();
        let cmsgs = &mut [];
        match guard.try_io(|inner| {
            match nix::sys::socket::sendmsg(
                inner.as_raw_fd(),
                buffer,
                cmsgs,
                flags,
                Some(socket_address),
            ) {
                Ok(read_bytes) => Ok(read_bytes),
                Err(would_block) => Err(std::io::Error::from_raw_os_error(would_block as i32)),
            }
        }) {
            Ok(res) => match res {
                Ok(read_bytes) => Ok(read_bytes),
                Err(e) => {
                    eprintln!("Error {}", e);
                    Ok(0)
                }
            },
            Err(e) => Err(e),
        }
    }

    pub async fn read(
        &'a self,
        buffer: &'a mut [IoSliceMut<'_>; 1],
        flags: MsgFlags,
    ) -> Result<usize, TryIoError> {
        buffer[0].fill(0);
        let mut guard = self.inner.readable().await.unwrap();

        match guard.try_io(|inner| {
            let sys_time = nix::time::clock_gettime(nix::time::ClockId::CLOCK_REALTIME).unwrap();
            println!("Real clock {:?}", sys_time);
            match nix::sys::socket::recvmsg::<()>(
                inner.as_raw_fd(),
                buffer,
                Some(&mut cmsg_space!(
                    nix::sys::socket::MsgFlags,
                    nix::sys::socket::TimestampingFlag,
                    nix::sys::socket::SockFlag
                )),
                flags,
            ) {
                Ok(result) => {
                    let mut ts = TimeSpec::new(0, 0);
                    let mut _thw = TimeSpec::new(0, 0);
                    let control_messages: Vec<ControlMessageOwned> = result.cmsgs().collect();

                    println!("Control message length = {}", control_messages.len());
                    for c in control_messages {
                        match c {
                            ControlMessageOwned::ScmTimestampsns(timestamps) => {
                                _thw = timestamps.hw_raw;
                                ts = timestamps.system;
                                println!("Timestamps {:?}", timestamps);
                            }
                            ControlMessageOwned::ScmRights(_) => println!("ScmRights"),
                            ControlMessageOwned::ScmCredentials(_) => println!("ScmCredentials"),
                            ControlMessageOwned::ScmTimestamp(_) => println!("ScmTimestamp"),
                            ControlMessageOwned::ScmTimestampns(_) => println!("ScmTimestampns"),
                            ControlMessageOwned::Ipv4PacketInfo(_) => println!("Ipv4PacketInfo"),
                            ControlMessageOwned::Ipv6PacketInfo(_) => println!("Ipv6PacketInfo"),
                            ControlMessageOwned::Ipv4OrigDstAddr(_) => println!("Ipv4OrigDstAddr"),
                            ControlMessageOwned::Ipv6OrigDstAddr(_) => println!("Ipv6OrigDstAddr"),
                            ControlMessageOwned::UdpGroSegments(_) => println!("UdpGroSegments"),
                            ControlMessageOwned::RxqOvfl(_) => println!("RxqOvfl"),
                            ControlMessageOwned::Ipv4RecvErr(a, b) => {
                                println!("Received ipv4 Err {:?} from {:?}", a, b);
                            }
                            ControlMessageOwned::Ipv6RecvErr(_, _) => println!("Ipv6RecvErr"),
                            _ => println!("Other"),
                        }
                    }

                    let soft_diff = diff_systime(ts, sys_time);

                    // let hw_diff = diff_systime(thw, sys_time);

                    if soft_diff != sys_time {
                        let delta = std::time::Duration::from(soft_diff).as_micros();
                        println!("Soft Delta is {}", delta);
                    }
                    // } else if hw_diff != sys_time {
                    // //     let delta = std::time::Duration::from(hw_diff).as_micros();
                    // //     println!("Hard Delta is {}", delta);
                    // // }

                    return Ok(result.bytes);
                }
                Err(errno) => {
                    match errno {
                        nix::errno::Errno::EAGAIN => println!("EAGAIN Error"),
                        _ => println!("Other error {:?}", errno),
                    }
                    let error = std::io::Error::from_raw_os_error(errno as i32);
                    Err(error)
                }
            }
        }) {
            Ok(res) => match res {
                Ok(read_bytes) => Ok(read_bytes),
                Err(_e) => {
                    println!("Error from socket {:?}", std::io::Error::last_os_error());
                    Ok(0)
                }
            },
            Err(e) => {
                println!("Guard error {:?}", std::io::Error::last_os_error());
                Err(e)
            }
        }
        // }
    }
}

impl<'a, T> AsRawFd for AsyncSocket<'a, T> {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl<'a, T> Drop for AsyncSocket<'a, T> {
    fn drop(&mut self) {
        let fd = self.inner.as_raw_fd();
        unsafe { nix::libc::close(fd) };
    }
}

fn diff_systime(first: TimeSpec, second: TimeSpec) -> TimeSpec {
    if second > first {
        second - first
    } else {
        first - second
    }
}
