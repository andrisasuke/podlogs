# Plan: kelompok environment variables dan Show more

Tanggal: 2026-11-13. Status: **diimplementasikan; verifikasi terarah lulus** (pengujian manual owner belum dilakukan).

## Tujuan

Ringkas daftar environment variables di Pod Details dengan mengelompokkan baris berdasarkan `EnvVar.source`. Setiap kelompok menampilkan lima item pertama dan menyediakan aksi untuk menampilkan seluruh itemnya.

## Kontrak tampilan dan interaksi

- Pertahankan disclosure `Environment Variables (N)` per container. N tetap jumlah seluruh `env_vars`, termasuk yang sedang tersembunyi; issue tidak dihitung sebagai variable.
- Urutan kategori tetap: `Direct`, `FieldRef`, `ResourceRef`, `ConfigMap`, `Secret`. Kategori kosong tidak dirender.
- Sumber lain yang belum dikenal tetap ditampilkan sebagai kelompok dengan label sumber aslinya, setelah kategori yang dikenal, diurutkan berdasarkan label. Jangan membuang baris atau mengubah model backend untuk normalisasi label.
- Judul kelompok menampilkan kategori dan jumlah seluruh barisnya, misalnya `Direct (12)`.
- Setiap kelompok awalnya menampilkan maksimal lima baris. Pertahankan urutan relatif dari daftar asal di dalam kelompok, termasuk duplikasi nama variable.
- Untuk kelompok dengan lebih dari lima baris, tampilkan `Show more (N more)` di bawah daftar; N adalah jumlah baris yang tersembunyi. Klik menampilkan semua baris sekaligus, bukan lima berikutnya.
- Setelah dibuka, aksi menjadi `Show less`; klik mengembalikan daftar ke lima baris. Kelompok dengan lima atau lebih sedikit baris tidak mempunyai aksi ini.
- Ekspansi independen per container dan kategori. Header kelompok tidak menambah disclosure lain; aksi Show more/Show less mengatur batas baris.
- Pertahankan badge sumber pada baris, copy, preview nilai, tooltip nama, referensi, dan `Show full value` yang sudah ada. Show more berbeda dari Show full value.
- Masalah resolusi tetap terlihat di bawah seluruh kelompok saat disclosure environment terbuka, termasuk pada container yang hanya mempunyai issue. Issue tidak terkena batas lima item.
- Keterangan nilai ConfigMap tetap tampil satu kali per container dengan syarat yang sama seperti sekarang.
- Kelompok kembali ke lima item saat scope context/namespace/pod berubah atau drawer ditutup. Refresh pada scope yang sama dan tutup/buka disclosure environment mempertahankan state kelompok.
- Show less tidak menghapus state detail nilai suatu baris. Jika baris itu disembunyikan lalu dimunculkan lagi, detailnya mengikuti state sebelumnya.

## Temuan kode dan desain

`src/app/pod_details.rs` saat ini merender `container.env_vars.iter().enumerate()` secara langsung. `PodDetailsView` memiliki `open_env` untuk disclosure container dan `expanded_env` untuk detail nilai. `reset_detail_state` mereset state ketika `pod_scope` berubah.

Perubahan cukup pada UI; tidak mengubah API, model, resolusi ConfigMap, atau akses Kubernetes. Tidak menambah dependency maupun fetch saat Show more diklik. Semua data sudah tersedia di `ContainerDetails.env_vars`.

Gunakan helper lokal untuk menghasilkan kelompok berisi referensi `(original_index, &EnvVar)`. Pertahankan indeks asal untuk ID baris, copy, referensi, dan detail nilai. Jangan memakai indeks posisi dalam kelompok atau nama variable sebagai pengganti: keduanya berpotensi mengarahkan interaksi ke baris lain atau bertabrakan pada nama duplikat.

Tambahkan state `expanded_env_groups: HashSet<(String, String)>` dengan key `(container_name, source)`. Tuple menghindari ambiguitas penggabungan string. Jangan memakai `expanded_env` untuk dua perilaku yang berbeda.

## Task 1 — Pengelompokan dan state

File: `src/app/pod_details.rs`.

1. Tambahkan konstanta batas preview bernilai 5 dan helper pengelompokan lokal yang mempertahankan indeks asal.
2. Tetapkan urutan kategori dikenal dan fallback kategori lain sesuai kontrak.
3. Tambahkan serta inisialisasi `expanded_env_groups` pada `PodDetailsView`.
4. Hubungkan state baru ke `reset_detail_state` dan seluruh pemanggilnya. Pertahankan state ketika scope tidak berubah.

## Task 2 — Render kelompok dan aksi ekspansi

File: `src/app/pod_details.rs`.

1. Ekstrak render satu baris environment menjadi method lokal agar markup, masking, copy, dan detail nilai tetap menggunakan satu implementasi.
2. Ganti daftar datar dengan daftar kelompok: judul, baris yang terlihat, lalu aksi Show more/Show less jika jumlah lebih dari lima.
3. Handler aksi hanya mengubah key kelompok terkait dan memanggil `cx.notify()`.
4. Pertahankan ID baris lama berbasis indeks asal. Tambahkan ID kelompok/aksi yang unik dan deterministik dari pasangan container dan source; gunakan ID terstruktur atau encoding tanpa benturan untuk label sumber yang tidak dikenal.
5. Pertahankan issue dan keterangan ConfigMap di luar daftar yang dibatasi. Pastikan container tanpa variable tetapi memiliki issue tetap terlihat.
6. Gunakan spacing dan warna tema yang sudah ada, serta lebar fleksibel agar judul dan aksi tetap berada di dalam drawer.

## Task 3 — Verifikasi terarah

File: modul tests pada `src/app/pod_details.rs`.

Tambahkan cakupan yang menguji perilaku pengguna dan risiko pengelompokan:

- Data campuran menghasilkan urutan kategori yang ditetapkan; urutan relatif, indeks asal, dan nama duplikat tetap terjaga. Kategori kosong tidak muncul; sumber tak dikenal tetap terlihat.
- Kasus batas 0, 5, dan 6 baris: tidak ada tombol untuk <=5; enam baris menampilkan lima lalu semua setelah Show more, dan kembali lima setelah Show less.
- Membuka satu kelompok tidak membuka kelompok lain atau kategori yang sama pada container lain.
- Klik copy dan Show full value pada baris yang posisi tampilnya berubah tetap menunjuk nilai asal yang benar. Verifikasi masking Secret dan unavailable tetap berlaku melalui tes regresi yang sudah ada.
- Reset scope menghapus state kelompok; notifikasi/refresh pada scope yang sama mempertahankannya. Pertahankan pemeriksaan issue-only dan layout drawer yang sudah ada.

Jalankan pemeriksaan otomatis saat implementasi:

```sh
rtk cargo test app::pod_details::tests
rtk cargo test
rtk cargo build
```

Pengujian aplikasi manual dilakukan owner:

1. Buka pod dengan beberapa kategori, termasuk kelompok yang memiliki lebih dari lima item.
2. Periksa hitungan kelompok, Show more/Show less, serta independensi antarkelompok dan antarcontainer.
3. Periksa nilai panjang/multiline, copy, Secret, dan ConfigMap unavailable.
4. Pastikan drawer tetap rapi dan bisa discroll setelah seluruh item dimunculkan.
5. Periksa refresh pada pod yang sama, tutup/buka disclosure, pindah pod, dan tutup/buka drawer sesuai kontrak state.

## Kriteria selesai

Seluruh kategori dan baris tetap dapat diakses, preview dibatasi lima per kelompok, ekspansi independen, serta perilaku nilai dan informasi kegagalan tetap berfungsi. Pemeriksaan otomatis lulus dan hasilnya dilaporkan; status pengujian manual owner dicatat secara terpisah.

Plan ini tidak menginstruksikan commit, PR, atau perubahan pada pekerjaan migrasi lainnya.

## Status implementasi (2026-11-13)

Task 1-3 selesai di `src/app/pod_details.rs`:

- `ENV_PREVIEW_LIMIT = 5`, urutan kategori `Direct`, `FieldRef`, `ResourceRef`, `ConfigMap`, `Secret`, lalu sumber tak dikenal diurutkan berdasarkan label. Helper `group_env_vars` mengembalikan `EnvGroup { source, rows: Vec<(original_index, &EnvVar)> }`.
- State `expanded_env_groups: HashSet<(String, String)>` di `PodDetailsView`, diinisialisasi di `new`, dan di-`clear` di `reset_detail_state` saat scope berubah (dipertahankan saat scope sama).
- Render satu baris diekstrak ke `PodDetailsView::env_row`, mempertahankan ID baris berbasis indeks asal (copy, mask, preview, tooltip, referensi, `Show full value`).
- Daftar datar diganti daftar kelompok: judul `Source (N)`, maksimal lima baris, lalu aksi `Show more (N more)` / `Show less`. ID kelompok/aksi deterministik tanpa benturan via `env_group_id`/`env_group_action_id` (encoding panjang).
- Issue dan keterangan ConfigMap tetap di luar daftar terbatas.

Hasil pemeriksaan otomatis: `cargo test app::pod_details::tests` 10 passed; `cargo test` 129 passed, 1 ignored; `cargo build` sukses tanpa warning.

Keterbatasan: pengujian manual GUI (drawer layout, scroll, refresh scope) dilakukan owner sesuai daftar di atas.

## Review follow-up: tiga celah tes (selesai diverifikasi)

Tiga tes baru di akhir `src/app/pod_details.rs` menutup tiga celah review. Hanya tes (`src/app/pod_details.rs`) dan status plan ini yang diubah; tidak ada perubahan kode produksi.

- `five_row_group_shows_all_without_action_and_empty_categories_absent`: kelompok tepat lima baris menampilkan kelimanya tanpa aksi Show more; kategori kosong (FieldRef/ResourceRef/ConfigMap/Secret) tidak dirender; container tanpa env (tanpa disclosure) dan container issue-only (issue tampil, tanpa grup) tetap benar.
- `group_expansion_preserved_on_refresh_and_reset_on_scope_change`: expand grup, lalu periodic tick (notify scope sama) dan tutup/buka disclosure environment mempertahankan ekspansi; tutup/buka drawer, pindah pod, pindah namespace, dan pindah context semuanya mereset ke lima baris. Menggunakan `FakeApi` dua pod (container `app` dan `web`) serta dua namespace/context.
- `configmap_group_show_less_keeps_sixth_row_detail_and_copy`: grup ConfigMap enam baris — Show more membuka baris keenam, detail nilai penuh dibuka, Show less menyembunyikannya, Show more memulihkannya dengan detail tetap terbuka, dan copy tetap memberi nilai penuh asli. Scroll drawer body (`pod-details`) sebelum interaksi karena aksi kelompok berada dekat lipatan drawer.

Hasil pemeriksaan otomatis (dengan `--locked`): `cargo test app::pod_details::tests` 13 passed; `cargo test` 132 passed, 1 ignored; `cargo build` sukses.

Keterbatasan: pengujian manual GUI (drawer layout, scroll, refresh scope) tetap dilakukan owner sesuai daftar di atas.

