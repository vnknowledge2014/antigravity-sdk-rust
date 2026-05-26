# Chương 3: Sử Dụng Nâng Cao (Advanced Usage)

Khi bạn muốn AI của mình làm được nhiều việc hơn là chỉ nói chuyện, đây là lúc cần đến các tính năng nâng cao: **Tools**, **MCP**, **Policies**, và **Triggers**.

## 1. Custom Tools (Tạo công cụ riêng)

Bạn có thể dạy Agent cách gọi hàm Rust của bạn bằng cách đăng ký một công cụ (`Tool`).

```rust
use antigravity_sdk::tools::tool_runner::{RegisteredTool, ToolRunner};
use serde_json::json;

let mut tool_runner = ToolRunner::new();

// Đăng ký Tool lấy thời tiết
tool_runner.register(RegisteredTool {
    name: "get_weather".to_string(),
    description: "Returns the current weather for a city.".to_string(),
    // Định nghĩa Schema (JSON Schema) cho đầu vào
    schema: Some(json!({
        "type": "object",
        "properties": {
            "city": { "type": "string" }
        },
        "required": ["city"]
    })),
    // Viết hàm logic (Handler)
    handler: Box::new(|args| {
        Box::pin(async move {
            let city = args.get("city").unwrap().as_str().unwrap();
            // Xử lý logic tại đây (có thể gọi API thật)
            Ok(json!(format!("Trời đang rất nắng ở {}.", city)))
        })
    }),
});
```

Sau khi cấu hình ToolRunner vào `AgentConfig`, Model có thể chủ động ra quyết định truyền biến `city` vào hàm này để lấy kết quả.

## 2. MCP Integration (Model Context Protocol)

Nếu bạn không muốn tự viết code, bạn có thể kết nối SDK với các [MCP Servers](https://modelcontextprotocol.io/) có sẵn. MCP Server là các module chạy độc lập có chứa hàng tá công cụ tiện lợi (ví dụ: File System, GitHub, PostgreSQL...).

Rust SDK sử dụng crate `rmcp` để kết nối mượt mà tới các MCP Servers:

```rust
use antigravity_sdk::types::McpStdioServer;
use antigravity_sdk::connections::local::LocalAgentConfig;

let mut config = LocalAgentConfig::default();
// Đăng ký một MCP Server chạy qua stdio
config.mcp_servers = vec![
    McpStdioServer {
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string(), "./my-folder".to_string()],
        env: None,
    }
];
```

## 3. Hooks và Policies (Chính sách An Toàn)

Khi cấp quyền cho Agent (đặc biệt là quyền thao tác với File hoặc Command), bạn cần một lớp bảo vệ. `HookRunner` cho phép bạn tạo ra các "nốt chặn" (Policy).

Ví dụ: Bạn có thể cấm toàn bộ, chỉ cho phép lệnh `ls`, và hỏi ý kiến người dùng nếu nó định chạy lệnh `git`:

```rust
use antigravity_sdk::hooks::policy::{allow, ask_user, deny, enforce};
use antigravity_sdk::connections::local::{CapabilitiesConfig, LocalAgentConfig};

let policies = vec![
    deny("*"),                    // Mặc định: Chặn mọi Tool
    allow("view_file"),           // Ngoại lệ: Cho phép xem file
    ask_user("run_command", ...), // Hỏi ý kiến trước khi chạy lệnh console
];

let mut config = LocalAgentConfig::default();
config.capabilities = Some(CapabilitiesConfig::default()); // Bật quyền ghi
config.policies = policies; // Áp dụng chính sách
```

Nếu Agent gọi hàm bị cấm, nó sẽ nhận được thông báo lỗi từ SDK và có thể tìm cách khác thay thế.

## 4. Triggers (Tác vụ ngầm)

Bạn muốn Agent tự động nhắn tin cho bạn khi một tiến trình chạy xong, hoặc báo thức bạn mỗi 5 phút? Hãy dùng Triggers:

```rust
use antigravity_sdk::triggers::{every, TriggerContext};
use std::sync::Arc;

let trigger = every(300.0, |ctx: Arc<TriggerContext>| async move {
    // Đoạn code này sẽ tự chạy ngầm mỗi 5 phút (300 giây)
    let _ = ctx.send("Hãy kiểm tra lại server xem có sập không!").await;
});

// Thêm vào LocalAgentConfig.triggers
```

## Tham khảo thêm

- **Mã nguồn mẫu:**
  - Tools: [`examples/getting_started/custom_tools.rs`](../examples/getting_started/custom_tools.rs)
  - MCP: [`examples/getting_started/mcp_tools.rs`](../examples/getting_started/mcp_tools.rs)
  - Policies & Hooks: [`examples/getting_started/policies.rs`](../examples/getting_started/policies.rs) và [`examples/getting_started/hooks.rs`](../examples/getting_started/hooks.rs)
  - Triggers: [`examples/getting_started/triggers.rs`](../examples/getting_started/triggers.rs)
- **Tài liệu cho AI (Skills):** Xem các file tương ứng trong thư mục [`skills/google-antigravity-sdk/examples/getting_started/`](../skills/google-antigravity-sdk/examples/getting_started/) hoặc tìm hiểu chi tiết lý thuyết tại thư mục [`skills/google-antigravity-sdk/references/`](../skills/google-antigravity-sdk/references/).

---

**Bước tiếp theo:** Để hiểu tại sao Rust SDK lại thiết kế mạnh mẽ và phức tạp đến vậy, cùng khám phá [Chương 4: Kiến trúc hệ thống (Architecture)](04_architecture.md).
