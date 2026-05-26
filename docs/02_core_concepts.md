# Chương 2: Khái Niệm Cốt Lõi (Core Concepts)

Trong chương này, chúng ta sẽ đi sâu vào các cấu trúc dữ liệu và thành phần chính cấu tạo nên Antigravity SDK.

## 1. Vòng đời của một Agent (`Agent Lifecycle`)

Struct `Agent` là "bộ não" quản lý toàn bộ vòng đời của hệ thống. Dưới đây là các hàm cơ bản bạn cần gọi:

1. **`Agent::new(config)`**: Cấu hình Agent. Lúc này Agent chưa làm gì, cũng chưa sinh ra kết nối mạng.
2. **`agent.start().await`**: Bắt đầu kết nối (qua WebSocket) tới Go `localharness`, kích hoạt các Hook và lắng nghe các Tool.
3. **`agent.chat(prompt).await`**: Hàm tiện ích để gửi 1 câu lệnh (`prompt`) và nhận lại toàn bộ kết quả sau khi Model đã phân tích, gọi Tools (nếu có), và tổng hợp lại.
4. **`agent.stop().await`**: Cắt đứt kết nối một cách an toàn và giải phóng tài nguyên.

## 2. Cuộc hội thoại (Conversation)

Dưới lớp vỏ bọc tiện lợi của `Agent`, thực chất mọi tương tác đều được quản lý bởi struct `Conversation`. `Conversation` lưu trữ lịch sử (`history`) toàn bộ ngữ cảnh trao đổi giữa bạn và mô hình AI.

Nếu bạn cần kiểm soát sâu hơn, bạn có thể tự mình khởi tạo `Conversation` thay vì dùng `Agent`:

```rust
use antigravity_sdk::connections::local::LocalConnectionStrategy;
use antigravity_sdk::conversation::Conversation;
use antigravity_sdk::tools::tool_runner::ToolRunner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tool_runner = ToolRunner::new();
    let mut strategy = LocalConnectionStrategy::new(Default::default(), tool_runner, None, None);
    
    // Tạo session hội thoại
    let mut conversation = Conversation::create(&mut strategy).await?;
    
    let response = conversation.chat("What files are here?").await?;
    println!("Response: {}", response.text().await);
    
    // Bạn có thể xem lịch sử các step
    println!("Total steps in history: {}", conversation.history().await.len());
    
    conversation.disconnect().await?;
    Ok(())
}
```

## 3. Bất đồng bộ (Async) với Tokio

Vì SDK này liên tục phải giao tiếp qua mạng (nhận/gửi JSON payload qua WebSocket), toàn bộ SDK được viết dựa trên mô hình **Bất đồng bộ (Async)** của Rust.

- Bất cứ hàm nào tốn thời gian chờ đợi mạng lưới đều mang chữ `async`.
- Phải có đuôi `.await` mỗi khi gọi chúng.
- Lỗi sẽ được bắt và trả về dưới dạng `Result<T, E>`.

## 4. Streaming Responses (Trực tiếp xả dữ liệu)

Lợi ích mạnh nhất của Antigravity là khả năng **Streaming**. Thay vì chờ AI nói hết 1 đoạn văn dài 1000 từ mới hiển thị, bạn có thể in ra từng chữ một (giống hệt trải nghiệm ChatGPT).

Hàm `agent.chat()` trả về một `ChatResponse`. Class này implement trait `Stream` của Rust (từ thư viện `futures`).

```rust
use futures::StreamExt;
use std::io::{self, Write};

// ... sau khi khởi tạo agent ...
let mut response = agent.chat("Hãy viết một bài thơ về ngôn ngữ Rust.").await?;

// Vòng lặp lấy từng token (chữ) ngay khi AI vừa nghĩ ra
while let Some(chunk) = response.next().await {
    print!("{}", chunk);
    io::stdout().flush()?;
}
println!();
```

Thậm chí bạn có thể Stream được cả quá trình "suy nghĩ" (Thoughts) của Model trước khi nó đưa ra câu trả lời bằng cách truy cập vào luồng nội bộ của `response`.

## Tham khảo thêm

- **Mã nguồn mẫu:** Hãy khám phá cách giao tiếp bất đồng bộ qua các ví dụ thực tế như [`examples/deep_dives/async_chat.rs`](../examples/deep_dives/async_chat.rs) hay cách streaming từng từ tại [`examples/getting_started/streaming.rs`](../examples/getting_started/streaming.rs).
- **Tài liệu cho AI (Skills):** Để AI hiểu sâu về các khái niệm này, xem thêm bài phân tích tại [`skills/google-antigravity-sdk/references/architecture.md`](../skills/google-antigravity-sdk/references/architecture.md) và [`skills/google-antigravity-sdk/examples/getting_started/streaming.md`](../skills/google-antigravity-sdk/examples/getting_started/streaming.md).

---

**Bước tiếp theo:** Khi bạn đã quen với cách AI phản hồi, hãy chuyển sang [Chương 3: Sử dụng Nâng cao (Advanced Usage)](03_advanced_usage.md) để cung cấp cho AI đôi tay (Tools) và công cụ (MCP)!
