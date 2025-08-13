# WebSocket MCP Server Example

This example demonstrates a fully functional MCP server using WebSocket transport.

## Features

- **WebSocket Transport**: Real-time bidirectional communication over WebSocket
- **Echo Tool**: Simple tool that echoes back messages with timestamps
- **Status Resource**: Provides server status information
- **Production-Ready**: Proper initialization handshake and message handling

## Running the Server

```bash
cargo run -p websocket-server
```

The server will start on `ws://127.0.0.1:8080/mcp`

## Testing the Server

You can test the WebSocket server using any WebSocket client or the MoCoPr WebSocket transport:

### Using a WebSocket Client

Connect to `ws://127.0.0.1:8080/mcp` and send MCP initialization:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": {
      "tools": {}
    },
    "clientInfo": {
      "name": "Test Client",
      "version": "1.0.0"
    }
  }
}
```

### Using MoCoPr WebSocket Transport

```rust
use mocopr_core::transport::websocket::WebSocketTransport;

let mut transport = WebSocketTransport::new("ws://127.0.0.1:8080/mcp").await?;
transport.send(r#"{"jsonrpc": "2.0", "method": "initialize", ...}"#).await?;
```

## WebSocket Implementation

This example showcases the **production-ready WebSocket implementation** that was completed to address the previously incomplete WebSocket message handling in the MoCoPr server framework.

### Key Features Fixed

- ✅ **Complete MCP Initialization Handshake** - Proper protocol negotiation
- ✅ **Message Parsing and Routing** - JSON-RPC 2.0 compliant message handling
- ✅ **Error Handling** - Comprehensive error responses with proper codes
- ✅ **Connection Management** - Graceful connection lifecycle handling
- ✅ **Bidirectional Communication** - Full duplex WebSocket messaging

The WebSocket transport is now **fully implemented** and ready for production use with comprehensive MCP protocol support.
