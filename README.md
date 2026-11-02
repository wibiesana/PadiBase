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

### 🔐 Auth Endpoints (Login, Register, Forgot & Reset Password, Profile)
- `POST /api/auth/register` (atau `POST /api/collections/:collection/register`) : Registrasi user baru dan langsung mengembalikan JWT `token` beserta `record` user.
- `POST /api/auth/login` (atau `POST /api/collections/:collection/auth-with-password`) : Login menggunakan `identity` / `email` / `username` dan `password`.
- `POST /api/auth/forgot-password` (atau `POST /api/collections/:collection/forgot-password`) : Buat token reset password (`reset_token`) berdasarkan email/username.
- `POST /api/auth/reset-password` (atau `POST /api/collections/:collection/reset-password`) : Reset password menggunakan `token` sekali pakai dan `password` baru.
- `POST /api/auth/change-password` (atau `POST /api/collections/:collection/change-password`) : Ganti password untuk user yang sedang login (`oldPassword`, `newPassword`).
- `GET /api/auth/me` & `POST /api/auth/refresh` (atau `GET /api/collections/:collection/me`) : Ambil profil user yang sedang login sekaligus perbarui JWT token.

### 🔄 Atomic Database Transactions, Batch & Custom SQL
- `POST /api/transactions` (atau `POST /api/batch`) : Jalankan banyak operasi (`create`, `update`, `delete`, atau `sql`) dalam **satu transaksi database atomik (`BEGIN` ... `COMMIT` / `ROLLBACK`)**. Jika ada satu langkah gagal, seluruh operasi sebelumnya otomatis di-*rollback*.
- `POST /api/sql` (atau `POST /api/sql/query`) : Eksekusi query SQL kustom berparameter (`SELECT ... JOIN ... GROUP BY`, dll.) khusus untuk Superadmin/Admin.
- `POST /api/custom/transfer` : Contoh endpoint bisnis kustom (`src/api/custom.rs`) menggunakan `state.db.begin()` (`DbTx`) untuk transfer nilai/stok/saldo antar record secara atomik.

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

## 🔗 Relasi Tabel & Eager Loading (Relations & Eager Loading)

PadiBase mendukung deteksi relasi antar-tabel secara otomatis tanpa perlu konfigurasi rumit, menggunakan mekanisme introspeksi database cerdas dan konvensi penamaan standar (kompatibel dengan gaya **Padi Rust**).

### 1. Cara Menentukan & Mendeteksi Relasi Antar-Tabel

PadiBase menentukan tabel relasi melalui 2 cara utama:

#### A. Konvensi Penamaan Kolom (*Naming Convention Fallback*)
Jika sebuah kolom berakhiran `_id` atau `Id` (misalnya `user_id`, `creator_id`, `kategori_id`, `author_id`):
1. **Pemotongan Kata Dasar**: PadiBase memotong akhiran `_id` atau `Id` untuk mendapatkan nama entitas dasarnya.
   - `user_id` $\rightarrow$ `user`
   - `author_id` $\rightarrow$ `author`
   - `category_id` $\rightarrow$ `category`
2. **Pencocokan Nama Tabel Otomatis**: PadiBase kemudian mencocokkan kata dasar tersebut dengan tabel-tabel yang tersedia di database:
   - Bentuk tunggal (*singular*): `user`, `author`, `category`
   - Bentuk jamak (*plural* `+s`): `users`, `authors`
   - Bentuk jamak (*plural* `+es`): `categories`, `classes`
3. Jika ditemukan tabel yang cocok, kolom tersebut **otomatis ditetapkan berelasi ke tabel target tersebut** dengan primary key `id`.

> [!TIP]
> **Relasi Profesional di Admin Dashboard (`/dashboard/`)**:
> Ketika Anda memilih tipe **`relation`** (atau mengklik tombol **`Relation ID`**):
> 1. Dashboard menampilkan dropdown **`References: [Target Table] . [Target Field]`** secara interaktif.
> 2. PadiBase otomatis mengintrospeksi tabel target dan **menyamakan tipe data fisik kolom** (misal jika `teachers.id` bertipe `INT` / `BIGINT`, kolom relasi otomatis dibuat `INT` / `BIGINT`, bukan teks acak).
> 3. PadiBase otomatis membuat constraint fisik **`FOREIGN KEY (...) REFERENCES ...`** di database SQL (SQLite, MySQL, PostgreSQL) saat tabel dibuat atau diperbarui!

#### B. Foreign Key Fisik Database (*Physical Foreign Key Constraint*)
Jika tabel Anda di SQLite, MySQL/MariaDB, atau PostgreSQL sudah memiliki constraint `FOREIGN KEY` formal:
```sql
CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id)
```
PadiBase langsung mendeteksinya secara native:
- **SQLite**: Menjalankan introspeksi `PRAGMA foreign_key_list('table_name')`.
- **MySQL / MariaDB**: Membaca metadata `INFORMATION_SCHEMA.KEY_COLUMN_USAGE`.
- **PostgreSQL**: Membaca `information_schema.table_constraints` & `constraint_column_usage`.

Tabel referensi target (`target_table`) dan target primary key (`target_pk`) langsung dihubungkan secara akurat sesuai skema database fisik Anda.

---

### 2. Auto Load Relations & Eager Loading (No N+1 Problem)

- **Bebas Masalah N+1 & Hemat Memori**: Eager loading menggunakan **1 batch query** (`WHERE id IN (...)`) per relasi, bukan query per-baris, sehingga sangat cepat dan hemat koneksi database.
- **Resolusi Kolom Display Heuristik**: PadiBase secara cerdas memilih kolom representatif untuk ditampilkan (seperti `name`, `username`, `title`, `nama`, `label`) di samping primary key target:
  ```json
  {
    "id": 1,
    "user_id": 5,
    "score": 95,
    "user": {
      "id": 5,
      "name": "Budi Santoso",
      "email": "budi@example.com"
    }
  }
  ```

---

### 3. Konfigurasi & Penggunaan di API

- **Pengaturan Global di `.env`**:
  ```env
  AUTO_LOAD_RELATIONS=false   # true: Eager load relasi untuk SEMUA tabel secara default
                              # false: Ikuti pengaturan per-tabel di Dashboard
  ```
- **Pengaturan Per-Tabel di Admin Dashboard (`/dashboard/`)**:
  - Buka tabel yang diinginkan di Dashboard &rarr; pilih tab/bagian **Table Settings**.
  - Aktifkan toggle **"Auto Load Relations (Eager Loading)"** (`ON` / `OFF`).
  - Ketika aktif, setiap query record tabel ini akan otomatis menyertakan data relasinya.
- **Query Parameter Eksplisit di API**:
  - `?with=user,category` : Muat relasi tertentu secara spesifik.
  - `?with=*` atau `?auto_load=true` : Muat seluruh relasi yang terdeteksi untuk request ini.
  - `?with=none` atau `?auto_load=false` : Matikan loading relasi untuk performa maksimal dan payload minimal.

---

### Realtime SSE Stream
- `GET /api/realtime` : Koneksi Server-Sent Events untuk live updates

### 📁 File Storage & Image Management (PocketBase / Supabase Style)

- **Struktur Storage Folder & File Unik**:
  Setiap file disimpan di direktori storage terisolasi dengan struktur rapi per-koleksi dan per-record unik, dilengkapi prefix hash acak untuk mencegah penimpaan nama file:
  ```text
  padi_data/storage/{collection_name}/{record_id}/{unique_prefix}_{original_filename}
  ```
- **Pembersihan Otomatis & File Override**:
  - Saat memperbarui record dengan file gambar baru (baik lewat `PATCH /records/:id` maupun `POST /files/...`), file lama yang tersimpan sebelumnya **otomatis dihapus (*override*) dari storage fisik**, sehingga ruang disk server tidak akan pernah dipenuhi oleh file yatim (*orphaned files*).
  - Jika sebuah record dihapus (`DELETE /records/:id`), seluruh folder storage dan file-file di dalamnya otomatis dibersihkan secara total.
- **Tampilan Thumbnail & Image Lightbox Modal**:
  - Pada **Tabel Data Dashboard**: Kolom bertipe file gambar menampilkan thumbnail visual interaktif.
  - **Full-Screen Lightbox**: Mengklik thumbnail gambar akan membuka modal viewer resolusi tinggi dengan informasi dimensi gambar (`width × height px`), tombol unduh langsung, dan tombol salin link URL file.
  - **Instant Live Preview Form**: Saat memilih file baru di drawer form record (`<input type="file">`), preview visual gambar langsung muncul secara instan sebelum menekan tombol simpan.

#### 🚀 Panduan Cara Upload File dari Client

Terdapat 2 cara bagi aplikasi client (Frontend Web, Mobile App, Postman) untuk mengunggah file:

##### 1. Cara Sekaligus (All-in-One: Create / Update Record)
Kirim `multipart/form-data` langsung ke endpoint record. PadiBase akan otomatis menyimpan file fisik dan mengisi kolom database terkait (misal `avatar`):

* **Endpoint**:
  * Buat Record Baru + File: `POST /api/collections/:collection/records`
  * Update Record + Ganti File: `PATCH /api/collections/:collection/records/:id`

* **Contoh JavaScript (Web / React / Vue)**:
  ```javascript
  const formData = new FormData();
  formData.append('name', 'Budi Santoso');
  formData.append('subject', 'Matematika');
  formData.append('avatar', fileInput.files[0]); // Kolom file

  const res = await fetch('http://127.0.0.1:8090/api/collections/teachers/records', {
    method: 'POST',
    headers: {
      'Authorization': 'Bearer ' + token // jika tabel wajib login
    },
    body: formData
  });
  const data = await res.json();
  console.log('Record & File tersimpan:', data);
  ```

##### 2. Cara API Khusus Upload File (`/api/files/...`)
Jika record sudah ada dan ingin mengunggah atau mengganti file secara spesifik:

* **Endpoint**: `POST /api/files/:collection/:record_id`
* **Contoh cURL**:
  ```bash
  curl -X POST "http://127.0.0.1:8090/api/files/teachers/rec_001" \
    -H "Authorization: Bearer YOUR_TOKEN" \
    -F "avatar=@/path/to/my_photo.png"
  ```
* **Response Output**:
  ```json
  {
    "message": "File(s) uploaded successfully.",
    "uploaded": [
      {
        "field": "avatar",
        "filename": "f1a2b3c4_my_photo.png",
        "original_name": "my_photo.png",
        "size": 182400,
        "mime": "image/png",
        "url": "/api/files/teachers/rec_001/f1a2b3c4_my_photo.png"
      }
    ],
    "record": {
      "id": "rec_001",
      "name": "Budi Santoso",
      "avatar": "f1a2b3c4_my_photo.png"
    }
  }
  ```

##### 3. Cara Menampilkan Gambar / File di Frontend:
Gunakan URL format `/api/files/{collection}/{record_id}/{filename}`:
```html
<!-- HTML / React / Vue -->
<img src="http://127.0.0.1:8090/api/files/teachers/rec_001/f1a2b3c4_my_photo.png" alt="Avatar">

<!-- Link Download Langsung (tambahkan ?download=1) -->
<a href="http://127.0.0.1:8090/api/files/teachers/rec_001/f1a2b3c4_my_photo.png?download=1">Unduh File</a>
```

### System Backups
- `GET /api/backups` : Daftar semua snapshot backup database
- `POST /api/backups` : Buat snapshot backup SQLite baru
- `GET /api/backups/:filename` : Download file backup `.db`
- `POST /api/backups/:filename/restore` : Restore database dari backup
- `DELETE /api/backups/:filename` : Hapus file backup
