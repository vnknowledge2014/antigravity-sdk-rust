# Chương 1: Nhập Môn Căn Bản (Getting Started)

Chào mừng bạn đến với **Antigravity SDK (Rust Edition)**! Tài liệu này sẽ hướng dẫn bạn cách cài đặt, cấu hình và chạy một AI Agent đơn giản nhất từ con số 0.

## 1. Yêu cầu hệ thống (Prerequisites)

Để sử dụng SDK này, bạn cần chuẩn bị:
1. **Rust Toolchain**: Tải và cài đặt Rust thông qua [rustup.rs](https://rustup.rs/). (Bao gồm `cargo` và `rustc`).
2. **Go Localharness**: SDK Antigravity giao tiếp với các model của Google thông qua một file thực thi (binary) viết bằng Go gọi là `localharness`. Bạn cần đảm bảo binary này đã được tải về và nằm trong biến môi trường `PATH`.
3. **API Key**: Một API Key hợp lệ của Gemini (Google AI Studio).

## 2. Tạo dự án mới

Khởi tạo một dự án Rust mới bằng Cargo:

```sh
cargo new my_agent
cd my_agent
```

Thêm Antigravity SDK và `tokio` (runtime bất đồng bộ của Rust) vào `Cargo.toml`. Vì SDK hiện chưa được public trên `crates.io`, bạn có thể trỏ đường dẫn tới local repo hoặc git repo:

```toml
[dependencies]
antigravity-sdk = { path = "../antigravity-sdk-python" } # Đường dẫn trỏ tới thư mục SDK
tokio = { version = "1", features = ["full"] }
```

## 3. Xin chào thế giới (Hello World)

Mở file `src/main.rs` và viết đoạn mã sau:

```rust
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::Agent;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // 1. Khởi tạo cấu hình Agent cơ bản (mặc định)
    let config = LocalAgentConfig::default();
    
    // 2. Tạo ra Agent
    let mut agent = Agent::new(config);
    
    // 3. Khởi động Agent (kết nối tới Localharness qua WebSocket)
    agent.start().await?;
    
    // 4. Gửi một tin nhắn và chờ phản hồi
    let prompt = "Xin chào, bạn có thể tóm tắt về ngôn ngữ Rust trong 2 câu được không?";
    println!("User: {}", prompt);
    
    let response = agent.chat(prompt).await?;
    println!("Agent: {}", response.text().await);
    
    // 5. Tắt Agent an toàn
    agent.stop().await?;
    
    Ok(())
}
```

## 4. Chạy thử

Đừng quên set API Key trước khi chạy:

```sh
export GEMINI_API_KEY="your_api_key_here"
cargo run
```

Nếu mọi thứ được cấu hình đúng, bạn sẽ thấy agent trả về 2 câu tóm tắt về ngôn ngữ Rust trên terminal.

> **💡 Lưu ý về tính An Toàn (Safety-First)**
> Mặc định, Antigravity Agent chạy ở chế độ **Read-only (Chỉ đọc)**. Nó sẽ KHÔNG thể tự động chạy command xoá file, sửa file hay thực hiện bất kỳ hành động ghi (Write) nào lên máy bạn trừ khi bạn chủ động cấp quyền thông qua `CapabilitiesConfig`. Chúng ta sẽ tìm hiểu điều này ở phần sau.

## Tham khảo thêm

- **Mã nguồn mẫu:** Bạn có thể xem toàn bộ mã nguồn có thể chạy được của phần Hello World tại [`examples/getting_started/hello_world.rs`](../examples/getting_started/hello_world.rs). Các ví dụ cơ bản khác nằm trong thư mục [`examples/getting_started/`](../examples/getting_started/).
- **Tài liệu cho AI (Skills):** Bạn có thể tham khảo thêm hướng dẫn dành riêng cho AI Agent (Skill) về phần này tại [`skills/google-antigravity-sdk/examples/getting_started/hello_world.md`](../skills/google-antigravity-sdk/examples/getting_started/hello_world.md).

---

**Bước tiếp theo:** Hãy chuyển sang [Chương 2: Khái niệm cốt lõi (Core Concepts)](02_core_concepts.md) để hiểu rõ cách Agent hoạt động dưới mui xe!
