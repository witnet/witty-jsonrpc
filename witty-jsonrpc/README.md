# witty-jsonrpc

An extensible JSON-RPC server that can listen over multiple transports at once.

## Supported transports
- HTTP
- TCP sockets
- WebSockets
- Whatever `T` you do `impl<H> Transport<H> for T where H: Handler`
