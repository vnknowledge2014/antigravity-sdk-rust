# Chương 4: Kiến Trúc Hệ Thống (Architecture)

Antigravity Rust SDK được thiết kế hoàn toàn theo mô hình **Rust Native**, nhắm đến tính an toàn tối đa (Memory Safe), và tốc độ xử lý nhanh gọn thông qua luồng dữ liệu bất đồng bộ.

## 1. Cấu trúc 3 Lớp (The 3-Layer Pattern)

Hệ thống được thiết kế dưới dạng 3 lớp trừu tượng hoá (Abstractions):

| Lớp (Layer) | Đối tượng chính (Structs) | Chức năng |
|:------|:------------|:--------|
| **Layer 1: Simplified** | `Agent`, `InteractiveCli` | Điểm truy cập cấp cao nhất. "Batteries-included" (gói gọn mọi thứ). Khởi tạo là chạy được ngay. Thích hợp cho 90% nhu cầu người dùng. |
| **Layer 2: Session** | `Conversation`, `HookRunner`, `ToolRunner` | Lớp quản lý vòng đời logic và lịch sử đàm thoại. Nó gom nhóm các chunks stream lại thành `Step`, tự động cập nhật lịch sử chat. |
| **Layer 3: Adapter** | `LocalConnection`, `ConnectionStrategy` | Lớp giao thức phần cứng. Nhiệm vụ duy nhất là gửi/nhận tín hiệu WebSocket, decode byte từ Protobuf thành cấu trúc Rust. |

## 2. Luồng IPC (Inter-Process Communication)

Antigravity SDK không trực tiếp thực hiện request HTTP lên Cloud của Gemini. Thay vào đó, mô hình của nó như sau:

```text
[ Của bạn (Rust SDK) ]  <--- (WebSocket / Protobuf) --->  [ Go Localharness ]  <--- (HTTP/gRPC) --->  [ Gemini API ]
```

1. **Rust SDK** gửi một cục byte (mã hoá bằng Protobuf qua thư viện `prost`) lên WebSocket.
2. Binary **Go Localharness** nhận được, nó sẽ tự động chịu trách nhiệm quản lý token, phân trang, và kết nối với Server API của Google.
3. Khi Server trả kết quả theo chuỗi stream, Go Localharness sẽ đẩy lại qua WebSocket xuống cho **Rust SDK**.
4. **Rust SDK** bóc tách Protobuf, tạo ra các đối tượng `StreamChunk` và đẩy lên cho `Conversation`.

Kiến trúc này giúp Rust SDK nhẹ nhàng, không dính líu đến các logic xác thực (Auth) phức tạp hay gRPC routing rắc rối.

## 3. An toàn bộ nhớ (Memory Safety) & Concurrency

Tất cả mã nguồn của SDK này bị ép buộc sử dụng macro:
```rust
#![forbid(unsafe_code)]
```
Tuyệt đối không có bất kỳ rủi ro nào liên quan đến lỗi rò rỉ bộ nhớ từ việc ép kiểu trực tiếp (như thường thấy ở các SDK C/C++ FFI). 

> [!NOTE]
> Mô hình `Arc<Mutex<T>>` cũ đã được **thay thế phần lớn** bởi **Actor Model** — xem [Chương 5](05_fp_architecture.md#3-actor-model). Actors chạy trong một `tokio::spawn` task duy nhất, sở hữu toàn bộ state, giao tiếp qua message passing. Zero-lock guarantee.

## 4. Kiến Trúc Functional Programming (FP)

SDK đã được refactor toàn bộ theo 4 trụ cột FP hiện đại:

### 4.1 Module `src/core/` — Functional Core

Chứa 5 module **pure functions** (không IO, không side effects):

| Module | Chức năng chính |
|:-------|:---------------|
| `pipeline.rs` | ROP types: `Pipeline<T>`, `PipelineError` |
| `agent_core.rs` | `validate_safety()`, `AgentPhase`, `AgentEvent` |
| `tool_core.rs` | `parse_wire_tool_call()`, `build_denial_result()` |
| `policy_core.rs` | `bucket_index()`, `matches_tool()`, `build_buckets()` |
| `step_core.rs` | `merge_usage()`, `add_option()` |

### 4.2 Module `src/actors/` — Actor Model

| Actor | Chức năng | Thay thế |
|:------|:---------|:---------|
| `StateActor` | Quản lý tất cả mutable state | 8 trường `Arc<Mutex<...>>` |
| `WriterActor` | Ghi bytes lên WebSocket | `Arc<Mutex<WsSink>>` |

### 4.3 Event Sourcing

- **AgentPhase**: State machine 5 trạng thái (`Created → Starting → Running → Stopping → Stopped`)
- **AgentEvent**: Nhật ký sự kiện append-only cho debugging và replay

> Để xem chi tiết đầy đủ, vui lòng đọc [Chương 5: Kiến Trúc FP](05_fp_architecture.md).

---

## Tham khảo thêm tổng hợp

- **Toàn bộ mã nguồn mẫu (Examples):** Nằm tại thư mục [`examples/`](../examples/), bao gồm các ví dụ căn bản (`getting_started`) và chuyên sâu (`deep_dives`).
- **Kiến trúc FP chi tiết:** Xem [Chương 5: Kiến Trúc Functional Programming](05_fp_architecture.md) — bao gồm ROP, FC-IS, Actor Model, Event Sourcing.
- **Thư mục Skills:** Để xem toàn bộ cấu trúc kiến thức được xây dựng nhằm "dạy" cho các AI Agent khác biết cách sử dụng Rust SDK này, vui lòng tham khảo file [`skills/google-antigravity-sdk/SKILL.md`](../skills/google-antigravity-sdk/SKILL.md) và các thư mục con bên trong nó.

