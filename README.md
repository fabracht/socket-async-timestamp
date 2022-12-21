# Async Socket with Kernel timestamps

This project is a test for using AsyncFd (from tokio) to wrap a socket file descriptor.

# Motivation

The motivation is to be able to access configuration that is not present in common user network libraries (such as tokio, std, socket2, smoltcp, etc), such as retrieving timestamp messages for RX and TX packets.
These values can be retrieved with a call to libc's `recvmsg` in a correctly configured socket (configured with SOF timestamping flags).
Tx timestamps can be retrieved from the Socket Error Queue, by accessing the socket's control messages.

# Current problems

Tx timestamps are stored (when MSG_ERRQUEUE is used) in the error queue, but polling of the socket only happens when a message is received buffer. This results in the following behavior:

Socket 1 sends message
Timestamping stuff is queued
... no polling is triggered and the message is not retrieved

Socket 1 receives the response
If recvmsg is called with MSG_ERRQUEUE:

- Socket is polled until all timestamps from the sent messages are received

If recvmsg is called without MSG_ERRQUEUE:

- Socket is polled and receives the payload of the response, but no data from error queue is read (which is the expected behavior)

If two calls to recvmsg are placed in the select! closure, it results in inconsistent behavior where sometimes you get error queued messages but sometimes you don't. This means that a second another call to recvmsg is required to receive the remaining Tx timestamps.

One way to fix it would be to poll the socket with recvmsg(...MSFG_ERRQUEUE) after sending the packet. But this is cumbersome and seems a bit hacky. Ideally I would like polling to happen when there is a message in the recv buffer and when there is no message in the recv buffer, but there is a queued entry in the Error queue.
