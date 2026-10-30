# 🌾 PadiBase (Backend-as-a-Service in Rust)

**PadiBase** adalah Backend-as-a-Service (BaaS) modern terinspirasi dari [PocketBase](https://pocketbase.io), dibangun dari nol menggunakan bahasa pemrograman **Rust**. PadiBase dirancang untuk performa tinggi, konkurensi masif, memori rendah, dan dikemas dalam **single executable binary** tanpa dependensi eksternal.

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
  Dashboard web yang modern dan responsif ter-embed langsung di dalam binary via `rust-embed` dan dapat diakses di `/dashboard/`.
- **📁 File Storage System**:
  Manajemen penyimpanan media & attachments dengan proteksi directory traversal.
- **📑 API Collection Exporter (Postman & REST Client)**:
  Ekspor seluruh koleksi atau per-koleksi ke format standar **Postman Collection v2.1.0** yang langsung dapat diimpor ke **Postman**, **Insomnia**, **Thunder Client**, **Bruno**, maupun **VS Code REST Client**, lengkap dengan environment variable (`base_url`, `auth_token`, id variable), dummy sample body otomatis, dan post-response test script.

---

## 🏗️ Struktur Proyek

```
PadiBase/
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
cargo run -- serve --db-url "mysql://root:password@127.0.0.1:3306/PadiBase"
cargo run -- serve --db-url "mariadb://root:password@127.0.0.1:3306/PadiBase"

# Menggunakan PostgreSQL
cargo run -- serve --db-url "postgres://postgres:password@127.0.0.1:5432/PadiBase"
```

Atau konfigurasi melalui file `.env` (kompatibel dengan **Padi Rust Api Framework**):
```env
DB_DRIVER=mysql        # sqlite | mysql | mariadb | pgsql | postgres
DB_HOST=127.0.0.1
DB_PORT=3306
DB_DATABASE=PadiBase
DB_USERNAME=root
DB_PASSWORD=secret
```
Akses di browser:
- **Admin Dashboard**: [http://127.0.0.1:8090/dashboard/](http://127.0.0.1:8090/dashboard/)
- **REST API**: [http://127.0.0.1:8090/api/](http://127.0.0.1:8090/api/)

### 2. Membuat Akun Admin via CLI

```bash
cargo run -- admin create admin@example.com "password_rahasia_123"
```

### 3. Ekspor API Collection untuk REST Client (Padi Rust Style)

Sama seperti di **Padi Rust**, Anda dapat mengekspor spesifikasi API Collection dalam format **Postman Collection v2.1.0** yang langsung dapat diimpor ke **Postman**, **Insomnia**, **Thunder Client**, **Bruno**, maupun **VS Code REST Client**:

```bash
# Ekspor semua koleksi & full system API collection ke folder ./api_collection
cargo run -- api-collection --force

# Ekspor koleksi tertentu saja (contoh: users)
cargo run -- api-collection --collection users --force

# Kustomisasi output direktori & base URL
cargo run -- api-collection --output ./my_collections --base-url "http://127.0.0.1:8090"
```

File yang dihasilkan di folder `./api_collection/`:
- `padi_base_api_collection.json` : Master koleksi lengkap mencakup seluruh endpoint sistem, admin gateway, schema management, backups, serta semua folder per-koleksi.
- `<collection>_collection.json` : Koleksi modular untuk masing-masing tabel (CRUD lengkap, endpoints gaya Padi Rust `/:collection/all`, realtime SSE, export, import, serta auto-save token/id via test scripts).

---

## 🔐 Alur Proteksi Admin Dashboard

1. Saat pertama kali dibuka di browser (`/dashboard/`):
   - Sistem memeriksa ketersediaan admin di backend (`/api/admins/has-admins`).
   - Jika belum ada admin, muncul layar **"Create First Superuser"**.
   - Jika sudah ada admin, muncul layar **"Admin Authentication"**.
2. Halaman **Collections Data**, **Records**, dan **Settings** terproteksi penuh dan baru bisa diakses setelah login berhasil.
3. Token tersimpan aman di sesi browser dan dapat di-logout kapan saja.

---

## 📡 Dokumentasi REST API

### Health Check
- `GET /api/health`

### 📑 REST Client API Collection Export
- `GET /api/api-collection` (atau `?download=true`) : Ekspor master Postman Collection v2.1.0 lengkap untuk seluruh endpoint sistem & semua koleksi.
- `GET /api/collections/:collection/api-collection` (atau `?download=true`) : Ekspor Postman Collection v2.1.0 khusus untuk satu koleksi.
- Tersedia juga tombol unduh langsung di **Admin Dashboard UI** (`/dashboard/`) pada menu dropdown JSON dan di dalam modal **API Explorer** (`Endpoints`).

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

### Dynamic Records CRUD & All Records (Padi Rust)
- `GET /api/collections/:collection/records?page=1&perPage=30&sort=-created` : List records terpaginasi (atau `?all=true` / `?perPage=-1` untuk semua data)
- `GET /api/collections/:collection/records/all` : Ambil **semua data** (List All) tanpa paginasi sama seperti di Padi Rust
- `GET /api/collections/:collection/all` : Shortcut endpoint List All
- `GET /api/:collection/all` (atau `/:collection/all`) : Route gaya Padi Rust untuk mendapatkan semua data (`?sort_by=id&order=desc&search=...`)
- `POST /api/collections/:collection/records` : Insert record baru (memerlukan user login secara default)
- `GET /api/collections/:collection/records/:id` : Ambil satu record
- `PATCH /api/collections/:collection/records/:id` : Update record (memerlukan user login secara default)
- `DELETE /api/collections/:collection/records/:id` : Hapus record (memerlukan user login secara default)

---

## 🔒 Security & API Access Rules (Default Logged-in Requirement)

Secara default, **PadiBase menerapkan proteksi keamanan berbasis autentikasi**:
1. **Default Create, Update, dan Delete**:
   - Operasi `create`, `update`, dan `delete` **secara default wajib dilakukan oleh user yang telah login** (`@request.auth.id != ""`, baik sebagai admin token maupun user token dari auth collection).
   - Request dari user anonymous / publik tanpa header `Authorization: Bearer <token>` akan ditolak dengan response **`401 Unauthorized`**.
2. **Otorisasi Berbasis Role (`users.role` / RBAC)**:
   - Mendukung pembatasan aksi spesifik berdasarkan role pengguna yang login (misalnya `creator`, `editor`, `admin`, dll.).
   - Role dibaca otomatis dari kolom `role` di tabel `users` saat user login dan disematkan ke dalam JWT Claims.
   - Format ekspresi rule yang didukung:
     - `@request.auth.role = 'creator'` atau `@request.auth.role in ['admin', 'creator']`
     - `admin, creator` (format shortcut fleksibel)
     - `role: creator, admin`
   - Superadmin / Admin selalu memiliki hak akses penuh (*bypass*).
3. **Kecuali User Mengubah Setting Sebaliknya di Dashboard (`/dashboard/`)**:
   - Jika ingin membuka akses publik untuk aksi tertentu (misalnya buku tamu, registrasi publik, form kontak publik), administrator dapat membuka drawer **Schema & Rules** di Dashboard dan mengubah rule field tersebut menjadi `public`.
   - Tersedia tombol preset cepat di Dashboard:
     - `Auth (Default)`: Wajib login (`@request.auth.id != ""`).
     - `Creator`: Khusus role creator atau admin (`@request.auth.role in ['admin', 'creator']`).
     - `Public`: Terbuka untuk umum / siapapun (`public`).
     - `Owner`: Hanya pemilik record yang bersangkutan (`id = @request.auth.id` atau `user_id = @request.auth.id`).
     - `Admin`: Hanya administrator (`admin`).
4. **List dan View**:
   - Operasi membaca data (`list`, `all`, `get_one`) secara default terbuka untuk publik (`public`) agar frontend dapat menyajikan data dengan cepat, kecuali jika rule list/view diatur khusus ke `auth`, role tertentu, atau `admin`.

---

## 🔗 Relations & Eager Loading (Padi Rust Style)

PadiBase automatically performs database introspection to discover foreign keys and naming conventions (e.g. `author_id` -> table `authors` or `author`).

### 1. Auto Load Relations & Eager Loading
- **No N+1 Problem & Memory-Efficient**: Eager loading uses **1 batch query** with `WHERE id IN (...)` per relation rather than per-row queries, preventing memory spikes and connection exhaustion.
- **Display Column Heuristic**: Automatically resolves representative display columns (e.g. `name`, `username`, `title`, etc.) matching Padi Rust conventions:
  ```json
  {
    "id": 1,
    "author_id": 5,
    "score": 95,
    "author": {
      "id": 5,
      "name": "John Doe"
    }
  }
  ```

### 2. Configuration & Usage
- **Global Option in `.env`**:
  ```env
  AUTO_LOAD_RELATIONS=false   # true: Eager load relations for ALL tables globally
                              # false: Follow per-table setting in dashboard
  ```
- **Per-Table Setting in Dashboard (`/dashboard/`)**:
  - Open **Schema & Rules** for any table.
  - Toggle **"Auto Load Relations (Eager Loading)"** (default per-table: `false`).
  - When global `.env` is `false`, any table with this toggle enabled will eager load its relations automatically.
- **Explicit Query Parameters**:
  - `?with=author,category` : Explicitly load specific relations.
  - `?auto_load=true` : Force load all relations for this request.
  - `?with=none` or `?auto_load=false` : Force disable relation loading for maximum speed and minimal payload.

---

### Realtime SSE Stream
- `GET /api/realtime` : Koneksi Server-Sent Events untuk live updates

### 📁 File Storage & Upload System
- `POST /api/files/:collection/:record_id` : Upload satu atau beberapa file attachment menggunakan `multipart/form-data` untuk record yang sudah ada.
- `GET /api/files/:collection/:record_id/:filename` : Download / streaming file statis dengan deteksi MIME type otomatis dan proteksi path traversal.
- `DELETE /api/files/:collection/:record_id/:filename` : Hapus file dari storage fisik dan kosongkan kolom file terkait di database.
- **Direct Multipart Record Creation/Update**: Endpoint `POST /api/collections/:collection/records` dan `PATCH /api/collections/:collection/records/:id` mendukung `multipart/form-data` secara langsung (upload file sekaligus mengisi data field dalam satu HTTP request).
- **Admin Dashboard UI**: Dukungan visual upload file (`<input type="file">`) pada Record Drawer, thumbnail preview gambar langsung pada tabel data, serta link unduh dan hapus file.

### System Backups
- `GET /api/backups` : Daftar semua snapshot backup database
- `POST /api/backups` : Buat snapshot backup SQLite baru
- `GET /api/backups/:filename` : Download file backup `.db`
- `POST /api/backups/:filename/restore` : Restore database dari backup
- `DELETE /api/backups/:filename` : Hapus file backup
