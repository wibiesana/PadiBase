# 🌾 Padi BaaS (Backend-as-a-Service in Rust)

**Padi BaaS** adalah Backend-as-a-Service (BaaS) modern terinspirasi dari [PocketBase](https://pocketbase.io), dibangun dari nol menggunakan bahasa pemrograman **Rust**. Padi BaaS dirancang untuk performa tinggi, konkurensi masif, memori rendah, dan dikemas dalam **single executable binary** tanpa dependensi eksternal.

---

## ⚡ Fitur Utama

- **🚀 Multi-Database Engine (SQLite, MySQL, MariaDB, PostgreSQL)**:
  Mendukung embedded SQLite (WAL Mode) secara default tanpa setup, maupun koneksi langsung ke **MySQL**, **MariaDB**, dan **PostgreSQL** via `DATABASE_URL` / `.env` / CLI `--db-url` seperti pada `padi_rust_api_framework`.
- **🔐 Protected Admin Gateway**:
  Wajib autentikasi admin sebelum mengakses Settings maupun Collections. Terdapat wizard pembuatan Superuser pertama saat inisialisasi awal.
- **🛡️ Modern Security & Cryptography**:
  Menggunakan **Argon2id** (standar emas hashing password) dan **JWT (JSON Web Token)** untuk otentikasi stateless yang aman.
- **📦 Dynamic Schema & Collections**:
  Buat dan kelola koleksi (Base Collection & Auth Collection) secara runtime. Skema database SQLite otomatis tersinkronisasi (`CREATE TABLE` dan `ALTER TABLE`).
- **⚡ Real-time SSE (Server-Sent Events)**:
  Setiap operasi perubahan data (`create`, `update`, `delete`) langsung dipancarkan secara real-time ke semua klien via SSE `/api/realtime`.
- **💾 Full System Backup & Restore**:
  Snapshot database SQLite atomik secara online via WAL checkpoint & `VACUUM INTO`. Mendukung unduh backup, restore, dan hapus snapshot.
- **💻 Embedded Admin Dashboard**:
  Dashboard web yang modern dan responsif ter-embed langsung di dalam binary via `rust-embed` dan dapat diakses di `/_/`.
- **📁 File Storage System**:
  Manajemen penyimpanan media & attachments dengan proteksi directory traversal.

---

## 🏗️ Struktur Proyek

```
padi_baas/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Library exports & App State
│   ├── main.rs             # Application entrypoint & Tracing logger
│   ├── core/               # Engine BaaS
│   │   ├── app.rs          # App state, directory storage & lifecycle
│   │   ├── auth.rs         # Argon2id hashing & JWT token generator
│   │   ├── db.rs           # SQLite pool & dynamic schema migrations
│   │   ├── record.rs       # Dynamic record CRUD & pagination
│   │   ├── schema.rs       # Collection & Schema field definitions
│   │   └── realtime.rs     # Pub/Sub hub tokio broadcast channel
│   ├── api/                # Axum REST & SSE Handlers
│   │   ├── router.rs       # Routing & CORS middlewares
│   │   ├── admins.rs       # Admin auth & superuser creation
│   │   ├── collections.rs  # Schema collection CRUD
│   │   ├── records.rs      # Dynamic records CRUD
│   │   ├── auth_records.rs # User/client login
│   │   ├── realtime.rs     # Realtime SSE stream
│   │   ├── files.rs        # Static file attachment serving
│   │   └── health.rs       # Health check endpoint
│   ├── ui/                 # Embedded Admin Dashboard
│   │   ├── mod.rs          # rust-embed handler
│   │   └── static/         # HTML5 + Tailwind SPA Dashboard
│   └── cli/                # Command Line Interface
│       └── commands.rs     # Clap commands (serve, admin create, version)
```

---

## 🚀 Cara Menjalankan

### 1. Menjalankan Server BaaS

```bash
# Default (Embedded SQLite di ./padi_data/data.db)
cargo run -- serve --http 127.0.0.1:8090 --dir ./padi_data

# Menggunakan MySQL / MariaDB
cargo run -- serve --db-url "mysql://root:password@127.0.0.1:3306/padi_baas"
cargo run -- serve --db-url "mariadb://root:password@127.0.0.1:3306/padi_baas"

# Menggunakan PostgreSQL
cargo run -- serve --db-url "postgres://postgres:password@127.0.0.1:5432/padi_baas"
```

Atau konfigurasi melalui file `.env` (kompatibel dengan **Padi Rust Api Framework**):
```env
DB_DRIVER=mysql        # sqlite | mysql | mariadb | pgsql | postgres
DB_HOST=127.0.0.1
DB_PORT=3306
DB_DATABASE=padi_baas
DB_USERNAME=root
DB_PASSWORD=secret
```
Akses di browser:
- **Admin Dashboard**: [http://127.0.0.1:8090/_/](http://127.0.0.1:8090/_/)
- **REST API**: [http://127.0.0.1:8090/api/](http://127.0.0.1:8090/api/)

### 2. Membuat Akun Admin via CLI

```bash
cargo run -- admin create admin@example.com "password_rahasia_123"
```

---

## 🔐 Alur Proteksi Admin Dashboard

1. Saat pertama kali dibuka di browser (`/_/`):
   - Sistem memeriksa ketersediaan admin di backend (`/api/admins/has-admins`).
   - Jika belum ada admin, muncul layar **"Create First Superuser"**.
   - Jika sudah ada admin, muncul layar **"Admin Authentication"**.
2. Halaman **Collections Data**, **Records**, dan **Settings** terproteksi penuh dan baru bisa diakses setelah login berhasil.
3. Token tersimpan aman di sesi browser dan dapat di-logout kapan saja.

---

## 📡 Dokumentasi REST API

### Health Check
- `GET /api/health`

### Admin Endpoints
- `GET /api/admins/has-admins` : Cek ketersediaan superuser
- `POST /api/admins` : Buat akun admin baru
- `POST /api/admins/auth-with-password` : Login admin (`identity`, `password`)

### Collections Management
- `GET /api/collections` : Daftar semua koleksi
- `POST /api/collections` : Buat koleksi baru (otomatis membuat tabel SQLite)
- `GET /api/collections/:name_or_id` : Dapatkan detail skema koleksi
- `PATCH /api/collections/:name_or_id` : Update skema koleksi
- `DELETE /api/collections/:name_or_id` : Hapus koleksi & drop tabel

### Dynamic Records CRUD
- `GET /api/collections/:collection/records?page=1&perPage=30&sort=-created` : List records
- `POST /api/collections/:collection/records` : Insert record baru (+ realtime broadcast)
- `GET /api/collections/:collection/records/:id` : Ambil satu record
- `PATCH /api/collections/:collection/records/:id` : Update record (+ realtime broadcast)
- `DELETE /api/collections/:collection/records/:id` : Hapus record (+ realtime broadcast)

### Realtime SSE Stream
- `GET /api/realtime` : Koneksi Server-Sent Events untuk live updates

### System Backups
- `GET /api/backups` : Daftar semua snapshot backup database
- `POST /api/backups` : Buat snapshot backup SQLite baru
- `GET /api/backups/:filename` : Download file backup `.db`
- `POST /api/backups/:filename/restore` : Restore database dari backup
- `DELETE /api/backups/:filename` : Hapus file backup
