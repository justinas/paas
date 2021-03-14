# Paas (processes as a service)

<!-- vim-markdown-toc GFM -->

* [Motivation](#motivation)
* [Worker](#worker)
* [Paasd](#paasd)
    * [API](#api)
        * [`Exec`](#exec)
        * [`GetLogs`](#getlogs)
        * [`GetStatus`](#getstatus)
        * [`Stop`](#stop)
        * [Error Conditions](#error-conditions)
    * [Security](#security)
        * [Transport](#transport)
        * [Authentication and authorization](#authentication-and-authorization)
        * [Other considerations](#other-considerations)
* [Paasc](#paasc)
* [Testing](#testing)

<!-- vim-markdown-toc -->

# Motivation

We want to provide a service that would let users run arbitrary Unix commands,
get information about their progress (via logs and exit code),
and allow to stop the processes if needed.

# Worker

`worker` is a Rust library that provides an async API
to spawn, control and inspect the processes.
It aims to provide all the control functionality
required by `paasd`, without tying it to the external API specifics.
This means it does not provide the gRPC protocol or transport.
Otherwise, functions provided by `worker` map almost 1-to-1
to endpoints provided by `paasd`.

# Paasd

`paasd` is a daemon that clients use to manage the processes.
It utilizes `worker` under the hood and provides a gRPC API
to access the functionality.

It also stores the information about executing and finished processes in-memory.
This in-memory store keeps the information for the lifetime of the daemon.

## API

Clients interact with `paasd` via a gRPC API. See `paas.proto` for RPC definition.

### `Exec`

`Exec` takes in the command line as an array of strings.
The first argument is the executable, the rest are arguments to the executable.
This is similar to the `argv` argument of `execv*` family.

On successfully spawning the process,
it is given a UUID, which is returned to the client
for use in future commands in regards to this process.

### `GetLogs`

`GetLogs` returns a stream of logs produced by the process with the given ID.
Both stdout and stderr are combined into a single stream.
`paasd` accumulates all logs produced by the process.
Upon the request, the stream will return all previous logs
and will then return messages as they are produced by the process.

`paasd` assumes that logs consist of valid UTF-8 textual data and uses line-based buffering.
Most Unix environments will use UTF-8
and most Unix-y utilities will produce textual logs with reasonably sized lines.

`GetLogs` prioritizes latency over throughput.
It will try to send multiple log lines in a single message, but this is not guaranteed.
In general, new log messages are sent as soon as possible, without additional buffering.

### `GetStatus`

`GetStatus` returns the status of the process with the given ID.
Currently, the response only indicates whether the process has exited,
in which case the response will contain either the signal number
(if the process has been killed by a signal), or the exit code otherwise.

### `Stop`

`Stop` stops the process with the given ID if it is still running.
It first tries to let the program terminate gracefully by sending a SIGTERM.
If the process is still running after a set amount of time, it will send a SIGKILL.
This mirrors the behavior of many process managers, for example,
[systemd](https://stackoverflow.com/questions/42978358/how-systemd-stop-command-actually-works).

It is a no-op to issue a `Stop` command for a process that has already been finished.
However, trying to `Stop` a process that is in the process of being stopped returns an error.

### Error Conditions

To indicate errors, built-in gRPC status codes have been chosen for simplicity's sake.
A human-readable error message will be provided in the response.

Examples of error conditions:

* Invalid arguments to `Exec` (empty array of strings)
* Error while spawning the process in `Exec` (e.g. the specified binary does not exist)
* Invalid process ID given to `GetLogs`, `GetStatus`, `Stop`
* Issuing `Stop` to a process when another `Stop` operation is in progress for that process.

## Security

### Transport

`paasd` uses TLS for transport security.
The clients only trust servers that produce a valid certificate
signed by *server certificate authority (CA)*.

As there is no requirement for compatibility with any legacy systems,
it is planned to support only the strongest cipher suites,
i.e. category "A" as outlined by [OWASP](https://cheatsheetseries.owasp.org/cheatsheets/TLS_Cipher_String_Cheat_Sheet.html#table-of-the-ciphers-and-their-priority-from-high-1-to-low-eg-18).
All of these ciphers provide both perfect forward secrecy
and authenticated encryption.
We choose to support multiple ciphers,
as this provides independent upgrade-paths for both client and server,
in case any of these ciphers get deprecated.

### Authentication and authorization

`paasd` uses mTLS to authenticate the clients via client certificates.
Upon connection, it is validated that the client has a valid certificate,
issued by the *client CA*.

We assume that every client has a unique certificate
and so use the certificates to uniquely identify clients.
Internally, a cryptographic hash of the client certificate could be used
to have a short, strong identifier for a client.

All authenticated users are allowed to spawn processes.
However, any given user is only authorized to manage (read logs, read status, stop)
the processes that they themselves have spawned.

For the PoC, both server and client certificates,
as well as server CA and client CA certificates
will be hardcoded and committed to the repository.

### Other considerations

For worker processes, no sandboxing is planned -
they run at the same privileges as the daemon.
This, at the very least, means that the operator should
run `paasd` under a user with low privileges.
Additional isolation measures, such as `chroot` or namespaces
are to be considered for production use.

# Paasc

`paasc` is a command line client to interact with `paasd`.
It communicates with `paasd` using the gRPC API,
using mTLS for transport security.

Subcommands of `paasc` are a 1-to-1 mapping to RPC functions provided by `paasd`.

```console
$ paasc exec my-script --flag1 --flag2
e260f390-c0ff-43cd-9714-53d2ece1c916
$ paasc status e260f390-c0ff-43cd-9714-53d2ece1c916
Status: running
$ paasc logs e260f390-c0ff-43cd-9714-53d2ece1c916
<live log output>
$ paasc stop e260f390-c0ff-43cd-9714-53d2ece1c916
$ paasc status e260f390-c0ff-43cd-9714-53d2ece1c916
Status: finished (exit code 0)
```

# Testing

Each component (`worker`, `paasd`, `paasc`) will have unit tests.
Basic integration tests will be written where viable.
E.g. `paasc` could be tested against an on-demand spawned instance of `paasd`
to ensure that the client-server protocol is upheld.
