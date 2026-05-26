# Google Antigravity SDK (Phiên bản Rust)

> **Lưu ý:** Đây là một bản chuyển đổi (port) không chính thức của Google Antigravity SDK từ Python sang Rust. Đây không phải là một dự án chính thức của Google.

Google Antigravity SDK là một SDK (viết bằng Rust) dùng để xây dựng các AI agent mạnh mẽ dựa trên công nghệ Antigravity và Gemini. Nó cung cấp một hạ tầng bảo mật, dễ mở rộng và quản lý trạng thái, giúp trừu tượng hóa các vòng lặp agent (agentic loop) để bạn có thể tập trung vào những gì agent *thực hiện* thay vì cách nó hoạt động.

## Cài đặt

Thêm SDK vào file `Cargo.toml` của bạn:

```toml
[dependencies]
antigravity-sdk = { git = "https://github.com/vnknowledge2014/antigravity-sdk-rust.git" }
tokio = { version = "1", features = ["full"] }
```

> [!IMPORTANT]
> Google Antigravity SDK dựa trên một tiến trình nhị phân (binary) localharness viết bằng Go, giao tiếp thông qua WebSocket và Protobuf. Hãy đảm bảo bạn đã thiết lập đúng harness này.

## Bắt đầu nhanh (Quickstart)

Hãy bắt đầu bằng cách chạy một trong các ví dụ trong thư mục [`examples/`](examples/), ví dụ như `hello_world`:

```sh
export GEMINI_API_KEY="your_api_key_here"
cargo run --example hello_world
```

## Các khái niệm cơ bản

### Simple Agent (Agent cơ bản)

Struct `Agent` là cách đơn giản nhất để bắt đầu. Nó quản lý toàn bộ vòng đời — kết nối công cụ, đăng ký hook, và các thiết lập an toàn mặc định.

```rust
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::Agent;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = LocalAgentConfig::default();
    let mut agent = Agent::new(config);
    
    agent.start().await?;
    
    let response = agent.chat("Những file nào đang có trong thư mục hiện tại?").await?;
    println!("{}", response.text().await);
    
    agent.stop().await?;
    Ok(())
}
```

### Phản hồi theo thời gian thực (Streaming Responses)

Để stream kết quả phản hồi theo thời gian thực (ví dụ, cho ứng dụng console hoặc giao diện mượt mà), bạn chỉ cần sử dụng object `ChatResponse` (có hỗ trợ trait `futures::stream::Stream`):

```rust
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::Agent;
use futures::StreamExt;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut agent = Agent::new(LocalAgentConfig::default());
    agent.start().await?;

    let mut response = agent.chat("Viết một bài thơ ngắn về vũ trụ.").await?;
    
    while let Some(chunk) = response.next().await {
        print!("{}", chunk);
        io::stdout().flush()?;
    }
    println!();
    
    agent.stop().await?;
    Ok(())
}
```

Mặc định, `Agent` chạy ở **chế độ chỉ đọc (read-only mode)** để đảm bảo an toàn. Hãy truyền `capabilities: CapabilitiesConfig::default()` nếu muốn kích hoạt mọi công cụ (bao gồm quyền ghi).

### Vòng lặp tương tác (Interactive Loop)

```rust
use antigravity_sdk::connections::local::{LocalAgentConfig, CapabilitiesConfig};
use antigravity_sdk::Agent;
use antigravity_sdk::utils::interactive::run_interactive_loop;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = LocalAgentConfig::default();
    config.capabilities = Some(CapabilitiesConfig::default());
    
    let mut agent = Agent::new(config);
    agent.start().await?;
    
    run_interactive_loop(&mut agent).await?;
    
    agent.stop().await?;
    Ok(())
}
```

### Sử dụng nâng cao với Conversation

Để kiểm soát hoàn toàn vòng đời kết nối, hãy sử dụng trực tiếp đối tượng `Conversation`:

```rust
use antigravity_sdk::connections::local::LocalConnectionStrategy;
use antigravity_sdk::conversation::Conversation;
use antigravity_sdk::tools::tool_runner::ToolRunner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tool_runner = ToolRunner::new();
    let mut strategy = LocalConnectionStrategy::new(
        Default::default(),
        tool_runner,
        None,
        None,
    );
    
    let mut conversation = Conversation::create(&mut strategy).await?;
    
    let response = conversation.chat("Có những file nào ở đây?").await?;
    println!("{}", response.text().await);
    
    println!("Tổng số bước: {}", conversation.history().await.len());
    
    conversation.disconnect().await?;
    Ok(())
}
```

## Tính năng

### Công cụ tùy chỉnh (Custom Tools)

Đăng ký các hàm hoặc closure của Rust dưới dạng công cụ để agent có thể gọi bằng `RegisteredTool`:

```rust
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::Agent;
use antigravity_sdk::tools::tool_runner::{RegisteredTool, ToolRunner};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut tool_runner = ToolRunner::new();
    let _ = tool_runner.register(RegisteredTool {
        name: "get_weather".to_string(),
        description: "Trả về thông tin thời tiết hiện tại cho một thành phố.".to_string(),
        schema: Some(json!({
            "type": "object",
            "properties": {
                "city": { "type": "string" }
            },
            "required": ["city"]
        })),
        handler: Box::new(|args| {
            Box::pin(async move {
                let city = args.get("city").unwrap().as_str().unwrap();
                Ok(json!(format!("Trời đang rất nắng ở {}.", city)))
            })
        }),
    });

    let config = LocalAgentConfig::default();
    let mut agent = Agent::new(config);
    // Lưu ý: Việc tích hợp Tool Runner sẽ tự động diễn ra thông qua hooks/config ở mức độ nâng cao
    agent.start().await?;
    agent.chat("Thời tiết ở Tokyo thế nào?").await?;
    agent.stop().await?;
    Ok(())
}
```

### Tích hợp MCP (Model Context Protocol)

Dễ dàng kết nối với các máy chủ [MCP](https://modelcontextprotocol.io/) bên ngoài và cung cấp các công cụ của chúng cho agent một cách native bằng cách sử dụng crate `rmcp`.

### Hooks và Policies

Kiểm soát hành vi của agent với một hệ thống khai báo chính sách mạnh mẽ nằm trong `HookRunner` hoặc thông qua `AgentConfig`.

## Tài liệu thành phần

Để xem tài liệu hướng dẫn chi tiết hơn, vui lòng tham khảo:

- [Bắt đầu (Getting Started)](docs/01_getting_started.md) — Cài đặt và cơ bản
- [Khái niệm cốt lõi (Core Concepts)](docs/02_core_concepts.md) — Agent, Conversation, Streaming
- [Sử dụng nâng cao (Advanced Usage)](docs/03_advanced_usage.md) — Custom Tools, MCP, Policies, Triggers
- [Kiến trúc (Architecture)](docs/04_architecture.md) — Khám phá cấu trúc bên trong bản Rust

## Giấy phép (License)

[Apache License 2.0](LICENSE)
