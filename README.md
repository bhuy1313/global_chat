# 🦀 Rust Global Chat

Hệ thống phòng chat trực tuyến toàn cầu (Real-time Global Chat) được xây dựng hoàn toàn bằng **Rust** (Axum + Tokio) và giao diện Web Client hiện đại.

---

## ✨ Tính Năng Nổi Bật

- **Tốc độ cao & Bất đồng bộ**: Sử dụng `axum` (v0.8) và runtime `tokio`, non-blocking I/O.
- **Kênh phát sóng (Broadcast)**: Phân phối tin nhắn tức thì qua `tokio::sync::broadcast` cho tất cả người dùng đang kết nối.
- **Lưu trữ đám mây bền vững (PostgreSQL)**:
  - Tích hợp chuẩn công nghiệp qua [`sqlx`](https://docs.rs/sqlx) hỗ trợ các Cloud DB miễn phí như **Neon.tech** hoặc **Supabase**.
  - Tự động tạo bảng `messages` và index khi khởi động.
  - Cơ chế **Dual-Mode / Graceful Fallback**: nếu không có `DATABASE_URL` hoặc DB mất kết nối, server sẽ tự động chuyển sang In-Memory Ring Buffer mà không bị gián đoạn hoạt động.
- **Quản lý danh tính**:
  - Tự động cấp biệt danh ngẫu nhiên (ví dụ `Guest_1234`).
  - Cho phép người dùng tùy ý đổi biệt danh; hệ thống sẽ lưu vào `localStorage` và tự động khôi phục khi mở lại tab.
  - Thông báo hệ thống khi đổi tên: `📢 OldName đã đổi tên thành NewName`.
- **Theo dõi số lượng người online**: Cập nhật thời gian thực số lượng kết nối đang mở.
- **Thông báo vào / rời phòng**: Tự động phát thông báo khi có người tham gia hoặc ngắt kết nối.
- **Giao diện Web hiện đại**:
  - Giao diện Dark theme sắc nét, phông chữ Plus Jakarta Sans.
  - Tự động cuộn thông minh (smart auto-scroll): nếu đang đọc lịch sử cũ sẽ không bị ép giật xuống đáy, hiển thị nút *Tin nhắn mới*.
  - Khay biểu tượng cảm xúc (Emoji Picker) nhanh.
  - Tự động kết nối lại khi rớt mạng (Exponential Backoff).
  - Không cần Node.js, Web Client được nhúng và phục vụ trực tiếp từ Rust server.

---

## 📁 Cấu Trúc Dự Án

```text
global_chat/
├── Cargo.toml               # Cấu hình dependency
├── src/
│   ├── main.rs              # Điểm khởi chạy, định tuyến HTTP & Middleware
│   ├── models.rs            # Struct định nghĩa ClientMessage & ServerMessage
│   ├── state.rs             # Bộ nhớ dùng chung (AppState, RingBuffer, Atomic Counter)
│   └── ws.rs                # WebSocket Upgrade Handler & Vòng lặp xử lý sự kiện
├── static/                  # Web Client tích hợp
│   ├── index.html           # Khung giao diện HTML5
│   ├── style.css            # Dark theme CSS hiện đại & responsive
│   └── app.js               # Logic WebSocket Client, tự động reconnect & render UI
└── README.md
```

---

## 🗄️ Cấu Hình Database Cloud (Neon.tech / Supabase)

1. Đăng ký tài khoản miễn phí tại **[Neon.tech](https://neon.tech)** (hoặc [Supabase](https://supabase.com)).
2. Tạo một dự án mới và copy chuỗi kết nối PostgreSQL (dạng `postgres://user:password@host/dbname?sslmode=require`).
3. Tạo file `.env` tại thư mục gốc của dự án:
   ```env
   DATABASE_URL=postgres://user:password@ep-sample.us-east-2.aws.neon.tech/neondb?sslmode=require
   ```
4. Chạy `cargo run`, server sẽ tự động tạo bảng `messages` và nạp lịch sử chat cũ!

---

## 🐳 Triển Khai Lên Cloud (Deploy với Docker)

Dự án đã bao gồm sẵn [`Dockerfile`](./Dockerfile) dạng Multi-stage siêu nhẹ (~30MB):

### Cách 1: Deploy lên Koyeb (Khuyên dùng - Free & Không bị sleep)
1. Push mã nguồn lên GitHub của bạn.
2. Đăng nhập [Koyeb.com](https://www.koyeb.com/) -> **Create App** -> Chọn **GitHub**.
3. Chọn repo `global_chat`, Koyeb sẽ tự nhận diện `Dockerfile`.
4. Tại phần **Environment Variables**, thêm biến:
   - `DATABASE_URL`: chuỗi kết nối từ Neon / Supabase.
   - `PORT`: `3000`
5. Nhấn **Deploy**! Koyeb sẽ tự động cấp một domain `https://<ten-app>.koyeb.app` bảo mật SSL.

### Cách 2: Deploy lên Render.com
1. Đăng nhập [Render.com](https://render.com/) -> **New Web Service** -> Kết nối GitHub repo.
2. Chọn Runtime: **Docker**.
3. Thêm biến môi trường `DATABASE_URL` trong mục **Environment**.
4. Nhấn **Create Web Service**.

---

## 🚀 Hướng Dẫn Chạy Ứng Dụng Cục Bộ

### 1. Khởi động Server
Mở terminal tại thư mục `global_chat`:

```bash
cargo run
```

Server sẽ lắng nghe tại:
- **Web UI**: [http://127.0.0.1:3000](http://127.0.0.1:3000)
- **WebSocket Endpoint**: `ws://127.0.0.1:3000/ws`
- **Health Check API**: [http://127.0.0.1:3000/health](http://127.0.0.1:3000/health)

### 2. Trải nghiệm
1. Mở trình duyệt tại [http://127.0.0.1:3000](http://127.0.0.1:3000).
2. Mở thêm một hoặc nhiều tab khác (hoặc trình duyệt ẩn danh) để giả lập nhiều người dùng trò chuyện cùng lúc.
3. Thử đổi biệt danh ở góc trên bên phải, gửi tin nhắn và emoji để trải nghiệm độ trễ gần như bằng 0 (real-time sub-millisecond).

---

## 📡 Chi Tiết Giao Thức WebSocket

### Client ➔ Server
1. **Gửi tin nhắn chat**:
   ```json
   { "type": "chat", "content": "Xin chào mọi người!" }
   ```
2. **Đổi biệt danh**:
   ```json
   { "type": "set_nickname", "nickname": "Rustacean" }
   ```

### Server ➔ Client
1. **Tin nhắn chat**:
   ```json
   {
     "type": "chat",
     "id": "1731140000-Rustacean",
     "nickname": "Rustacean",
     "content": "Xin chào mọi người!",
     "timestamp": "10:20"
   }
   ```
2. **Sự kiện hệ thống**:
   ```json
   {
     "type": "system",
     "content": "Rustacean đã tham gia phòng chat",
     "timestamp": "10:20"
   }
   ```
3. **Số lượng online**:
   ```json
   { "type": "user_count", "count": 3 }
   ```
4. **Lịch sử chat ban đầu**:
   ```json
   { "type": "history", "messages": [...] }
   ```
