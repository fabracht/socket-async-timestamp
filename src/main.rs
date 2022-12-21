use std::{
    io::{IoSlice, IoSliceMut},
    str::FromStr,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    time::Duration,
};
mod error;
mod socket;

use nix::sys::socket::{
    bind, setsockopt,
    sockopt::{self},
    AddressFamily, MsgFlags, SockFlag, SockProtocol, SockType, SockaddrIn, TimestampingFlag,
};
use socket::AsyncSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let local_sock_addr = SockaddrIn::from_str("0.0.0.0:6790").unwrap();
    let local_sock_addr1 = SockaddrIn::from_str("192.168.1.84:44581").unwrap();

    let send_sock_addr = SockaddrIn::from_str("192.168.1.123:6790").unwrap();

    let rsock = nix::sys::socket::socket(
        AddressFamily::Inet,
        SockType::Datagram,
        SockFlag::all(),
        SockProtocol::Udp,
    )?;

    let ssock = nix::sys::socket::socket(
        AddressFamily::Inet,
        SockType::Datagram,
        SockFlag::all(),
        SockProtocol::Udp,
    )?;

    // let sock_txtime = sock_txtime {
    //     clockid: nix::time::ClockId::CLOCK_MONOTONIC.as_raw(),
    //     flags: SOF_TXTIME_REPORT_ERRORS,
    // };
    setsockopt(rsock, sockopt::Timestamping, &TimestampingFlag::all())?;
    setsockopt(ssock, sockopt::Timestamping, &TimestampingFlag::all())?;
    // setsockopt(ssock, sockopt::ReuseAddr, &true)?;
    // setsockopt(rsock, sockopt::ReuseAddr, &true)?;

    // setsockopt(ssock, sockopt::TxTime, &sock_txtime)?;
    bind(ssock, &local_sock_addr1)?;
    bind(rsock, &local_sock_addr)?;

    let recv_socket: AsyncSocket<i32> = AsyncSocket::new(rsock)?;
    let send_socket: AsyncSocket<i32> = AsyncSocket::new(ssock)?;
    let atomic_i = Arc::new(AtomicU8::new(1));

    let mut read_buf = [0u8; 1024];
    let mut iov2 = [IoSliceMut::new(&mut read_buf)];

    // let mut rbuf1 = [0u8; 1024];
    let mut rbuf2 = [0u8; 1024];
    // let mut iov3 = [IoSliceMut::new(&mut rbuf1)];
    let mut iov4 = [IoSliceMut::new(&mut rbuf2)];

    loop {
        tokio::select! {
            read = recv_socket.read(&mut iov2, MsgFlags::empty()) => {
                match read {
                    Ok(v) => {
                        println!("Recv sock Received {} bytes in mes {:?}", v, iov2[0].iter().take(v).collect::<Vec<&u8>>());
                        let i = atomic_i.load(Ordering::Relaxed);
                        let sbuf: Vec<u8> = (1u8..=i).map(|el| el).collect();

                        let iov1 = [IoSlice::new(&mut sbuf.as_slice())];
                        tokio::time::sleep(Duration::from_millis(15)).await;

                        let _ = recv_socket.write_to(&iov1, &local_sock_addr1).await;

                    },
                    Err(e) => println!("Recv Err {:?}", e),
                }
            },
            _tick = tokio::time::sleep(Duration::from_millis(500)) => {
                // println!("Tick");
                let i = atomic_i.load(Ordering::Relaxed);
                if i == 3 {
                    continue;
                    // In case you want the sending to last forever

                    // atomic_i
                    //     .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| Some(n - n))
                    //     .unwrap();
                    // break;
                }
                let sbuf: Vec<u8> = (1u8..=i).map(|el| el).collect();
                let iov1 = [IoSlice::new(&mut sbuf.as_slice())];
                let _ = send_socket.write_to(&iov1, &send_sock_addr).await;
                // Calling read here results in a deadlock
                println!("Message {} sent", i);
                atomic_i.fetch_add(1, Ordering::Relaxed);
            },
            read2 = send_socket.read(&mut iov4, MsgFlags::empty()) => {
                match read2 {
                                Ok(v) => {
                                    println!("Send sock Received {} bytes in mes {:?}", v, iov4[0].iter().take(v).collect::<Vec<&u8>>());
                                    // This second read call is done to retrieve any messages present in the Error queue (timestamps are there)
                                    // match send_socket.read(&mut iov3, MsgFlags::MSG_ERRQUEUE).await {
                                    //     Ok(v) => println!("Send sock Received from Error queue {} bytes in mes {:?}", v, iov3[0].iter().take(v).collect::<Vec<&u8>>()),
                                    //     Err(e) => println!("Send Err {:?}", e),
                                    // }
                                 },
                                Err(e) => println!("Send Err {:?}", e),
                            }
            },
            // Adding this entry results in very inconsistent behavior for receiving Tx timestamps
            // read1 = send_socket.read(&mut iov3, MsgFlags::MSG_ERRQUEUE) => {
            //     match read1 {
                                // Ok(v) => println!("Send sock Received from Error queue {} bytes in mes {:?}", v, iov3[0].iter().take(v).collect::<Vec<&u8>>()),
                                // Err(e) => println!("Send Err {:?}", e),
            //                 }
            // },

        }
        println!("\n")
    }

    // Ok(())
}
