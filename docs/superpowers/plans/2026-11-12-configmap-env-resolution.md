# Plan: tampilkan isi ConfigMap pada environment variable Pod Details

Tanggal: 2026-11-12. Status: **implementasi Tasks 1–3 selesai; pemeriksaan manual owner masih tertunda**.

## Tujuan dan batas cakupan

Pod Details menampilkan setiap key/value ConfigMap yang dirujuk oleh environment container, dengan sumber yang dapat ditelusuri. Mencakup:

- `envFrom[].configMapRef`: ekspansi semua pasangan dalam `ConfigMap.data`, termasuk `envFrom.prefix`.
- `env[].valueFrom.configMapKeyRef`: nilai key terpilih, memakai nama variable dari deklarasi `env.name`.
- Nilai kosong, panjang, dan multiline dapat dibaca serta disalin lengkap.
- Kegagalan satu ConfigMap tidak menggagalkan informasi pod, container, maupun ConfigMap lain.

Tetap memakai Rust/gpui, kube 0.98 dan k8s-openapi 0.24. Tidak menambah dependency. Tidak membaca isi Secret; masking `<secret>` tetap berlaku. Tidak melakukan exec ke container, tidak mengubah cluster atau RBAC, dan tidak menambah resolusi FieldRef/ResourceRef, init container, ephemeral container, maupun ConfigMap yang hanya dipasang sebagai volume.

Ini merupakan fitur tambahan yang disengaja terhadap React lama, bukan syarat parity migrasi.

## Temuan pada kode saat ini

- `src/k8s/pods.rs::get_pod_details` mengambil Pod lalu memanggil `get_container_details(&pod)` secara sinkron.
- `get_container_details` hanya membaca deklarasi Pod. `configMapKeyRef` menjadi `nama:key`; `configMapRef` menjadi satu baris `* (all keys)` / `from ConfigMap: nama`.
- `EnvVar` hanya berisi `name`, `value`, `source`; `ContainerDetails` belum mempunyai informasi kegagalan resolusi environment.
- `src/app/pod_details.rs` menampilkan `env_vars`, memakai nilai selectable dengan `max_lines(1)` dan parent `overflow_hidden`. Resolusi backend saja belum cukup untuk membaca nilai panjang dengan nyaman.
- `src/api.rs` sudah membatasi keseluruhan pengambilan Pod Details dengan timeout 30 detik.
- Mock HTTP Kubernetes tersedia sebagai pola di `src/k8s/logs_tests.rs`, dengan `tower::service_fn` dan `kube::Client::new`.

## Kontrak perilaku

### Apa yang ditampilkan

Tampilan ini adalah **deklarasi environment beserta nilai ConfigMap saat pengambilan data**, bukan snapshot environment proses yang sedang berjalan. Tambahkan keterangan singkat ketika ada sumber ConfigMap:

> ConfigMap values reflect the current configuration and may differ from a running container.

Nilai ditampilkan sesuai isi `ConfigMap.data`, tanpa ekspansi `$(OTHER_VAR)`, shell interpolation, parsing file, atau konversi `binaryData`. Key seperti `application.yaml` tetap satu key dengan seluruh isi teksnya; jangan memecah isi file menjadi variable-variable baru.

Jangan mengklaim hasil sebagai effective environment: tampilkan seluruh deklarasi/sumber dan pertahankan duplikasi nama dengan identitas baris serta referensi sumber yang berbeda. Ini penting ketika beberapa `envFrom`, Secret, dan `env` saling menimpa. Tidak perlu mengambil Secret untuk menghitung prioritas. Urutan kelompok mengikuti sumber yang dirujuk; key dari satu ConfigMap diurutkan secara deterministik. Pertahankan urutan deklarasi `env` yang sudah ada.

Validitas nama environment berbeda antarversi/feature gate cluster. Jangan menambahkan filter regex berbasis asumsi versi klien. Karena UI menampilkan sumber konfigurasi, semua key tekstual ditampilkan, tanpa menjanjikan seluruh key berhasil diinjeksikan oleh kubelet.

### Keberhasilan

Contoh ConfigMap `app-config` berisi `HOST=service.internal`, `PORT=8080`:

| Deklarasi | Baris hasil | Referensi sumber |
| --- | --- | --- |
| `envFrom.configMapRef: app-config` | `HOST=service.internal`, `PORT=8080` | `app-config:HOST`, `app-config:PORT` |
| sumber sama dengan prefix `APP_` | `APP_HOST=service.internal`, `APP_PORT=8080` | key asli tetap `HOST`, `PORT` |
| `env.name=SERVER_PORT`, keyRef `app-config:PORT` | `SERVER_PORT=8080` | `app-config:PORT` |

`data: null` atau data kosong merupakan keberhasilan dengan nol key. String kosong adalah nilai sah; tampilkan penanda `(empty)` terpisah dari data agar copy tetap menghasilkan string kosong. `binaryData` tidak diperlakukan sebagai environment tekstual.

### Gagal atau tidak tersedia

- Bedakan akses ditolak (403), objek tidak ditemukan (404), key tidak ditemukan, timeout, dan error lain.
- Catat nama ConfigMap, key bila ada, container, `optional`, dan alasan yang ramah pengguna. Jangan memasukkan isi ConfigMap ke log/error.
- `optional: true` + objek/key hilang: tampilkan catatan optional source/key unavailable; jangan membuat nilai kosong palsu atau menyebut kegagalan pod.
- `optional` tidak mengabaikan 403, masalah jaringan, atau timeout.
- Gagal pada `envFrom`: jangan tampilkan `* (all keys)` sebagai keberhasilan; tampilkan sumber yang belum terurai dalam daftar masalah.
- Gagal pada `configMapKeyRef`: pertahankan nama variable, tampilkan status unavailable dan referensi aslinya. Jangan mencampur string error ke dalam nilai yang bisa disalin.
- Tidak mencoba membaca isi Secret. Referensi Secret dan `<secret>` tetap seperti sekarang.

## Desain implementasi

### Model yang kompatibel dengan fixture lama

Di `src/models/pod.rs`:

- Tambahkan `EnvVar.reference: Option<String>` dengan `#[serde(default)]`, misalnya `app-config:PORT`.
- Tambahkan `EnvVar.resolution` berupa enum serde/default: `Declared` (default untuk data lama/direct/FieldRef), `ConfigMapResolved`, `Unavailable`. Ini membedakan error dari nilai kosong tanpa memeriksa string presentasi.
- Tambahkan `ContainerDetails.env_issues: Vec<EnvResolutionIssue>` dengan `#[serde(default)]`.
- `EnvResolutionIssue` memuat nama sumber, key opsional, optional flag, dan reason terstruktur (`Forbidden`, `NotFound`, `MissingKey`, `Timeout`, `Other`). UI memformat pesan dari reason; tidak perlu membawa respons HTTP mentah.

Perbarui seluruh literal struct yang terpengaruh secara eksplisit; jangan menambahkan `Default` ke semua model hanya untuk memudahkan tes. API `Api::pod_details` dan endpoint FakeApi tidak berubah.

### Backend dan batas I/O

Pisahkan I/O dari transformasi:

1. `get_pod_details` tetap memperoleh client dari context yang dipilih, lalu memanggil seam internal `get_pod_details_with_client(client, namespace, pod_name)` agar dapat diuji melalui HTTP mock.
2. Ambil Pod seperti sekarang; kegagalan langkah ini tetap menggagalkan Pod Details.
3. Kumpulkan nama ConfigMap unik dari dua jenis referensi pada container yang saat ini didukung.
4. Ambil hanya nama-nama tersebut memakai `Api<ConfigMap>::namespaced(client.clone(), namespace).get(name)`. Jangan `list` seluruh ConfigMap namespace atau mengambil namespace/context lain.
5. Dedup per request Pod Details, termasuk bila satu sumber dipakai beberapa container atau lewat kedua jenis referensi. Simpan keberhasilan maupun kegagalan dalam map lokal; jangan membuat cache global lintas context/namespace/polling.
6. Batasi concurrency ke 4, timeout per GET ke 3 detik, serta deadline bersama fase resolusi ke 8 detik. Semua sumber yang belum selesai saat deadline menjadi `Timeout`; informasi Pod yang sudah didapat tetap dikembalikan. Gunakan futures/tokio yang sudah tersedia, tanpa detached task.
7. Helper murni mengubah Pod + map hasil lookup menjadi `Vec<ContainerDetails>`, mengekspansi ConfigMap dan mengisi issue. Status, resources, ports, kondisi, dan metadata pod tetap melalui alur yang sama.

Map lookup memakai nama karena context dan namespace sudah dibatasi satu request. Setiap container tetap menghasilkan referensi/prefix/nama variable sendiri. Refresh berikutnya membaca ulang ConfigMap; tidak ada fetch dari render gpui atau saat membuka disclosure environment.

### UI Pod Details

- Pertahankan gaya kartu, disclosure, warna nama variable, badge sumber `ConfigMap`, serta masking Secret.
- Tampilkan referensi `configmap:key` melalui tooltip/detail baris agar asal nilai dapat ditelusuri tanpa melebarkan drawer.
- Baris nilai panjang/multiline dapat dibuka menjadi detail penuh di dalam drawer dengan wrap dan scroll yang dibatasi; nilai penuh selectable dan memiliki aksi Copy memakai string asli. Preview satu baris tetap ringkas saat tertutup.
- Gunakan identitas container + indeks baris untuk ekspansi/copy; jangan memakai nama variable sebagai ID unik karena duplikasi bisa terjadi.
- Nilai unavailable menampilkan status dan sumber; aksi copy nilai tidak tersedia. Nilai kosong yang berhasil diambil tetap bisa disalin.
- Render daftar `env_issues` di disclosure environment per container, termasuk bila `env_vars` kosong. Jangan menyembunyikan seluruh bagian hanya karena tidak ada variable yang berhasil diperoleh.
- Hitungan menyesuaikan baris variable yang berhasil diekspansi; issue bukan variable tambahan. Placeholder Secret yang sudah ada tetap mengikuti perilaku lama dan tidak diekspansi pada fitur ini.
- Reset state detail/copy saat scope pod berubah; jangan membocorkan nilai dari pod sebelumnya. Jumlah besar key jangan menghasilkan pemotongan data diam-diam; ukur layout responsif dengan fixture yang cukup besar.

## Urutan tugas dan acceptance

### Task 1 — Model dan resolver murni

File: `src/models/pod.rs`, `src/k8s/pods.rs`; ekstrak helper ke `src/k8s/pod_env.rs` hanya jika pemisahan membuat modul lebih jelas (tambahkan deklarasi di `src/k8s/mod.rs`).

- [x] Tambahkan model metadata resolusi/issue beserta tes deserialisasi fixture lama.
- [x] Tes ekspansi `envFrom`, prefix, keyRef dengan alias nama variable, nilai kosong, multiline, dan data kosong.
- [x] Tes duplikasi nama tetap terlihat dengan sumber masing-masing; urutan deterministik dan prefix tidak mengubah key lookup.
- [x] Tes objek/key hilang, optional/non-optional, dan referensi sumber tetap tersedia.
- [x] Tes `binaryData` tidak didekode sebagai environment; key dengan titik tidak dipecah/dibuang diam-diam. *(Resolver membaca `data` saja; key bertitik ikut dites.)*
- [x] Tes Direct/FieldRef/ResourceRef/Secret mempertahankan output, source, reference, dan status declared yang diharapkan.
- [x] Implementasikan helper murni ekspansi ConfigMap.

### Task 2 — Integrasi Kubernetes dan partial failure

File: `src/k8s/pods.rs`, baru `src/k8s/pods_tests.rs` mengikuti pola mock HTTP logs.

- [x] Uji endpoint Pod dan ConfigMap yang benar pada namespace/context client yang diberikan.
- [x] Sumber berulang lintas container/jenis referensi menghasilkan tepat satu GET per nama.
- [x] Tidak ada GET Secret atau LIST ConfigMap.
- [x] Respons mixed success/403/404 mempertahankan Pod Details dan sumber yang berhasil.
- [x] Dengan jam Tokio terkontrol, assert timeout GET tunggal tepat 3 detik dan deadline bersama tepat 8 detik; pastikan concurrency tepat 4 dan request aktif sudah dibatalkan.
- [x] Pod tanpa referensi ConfigMap tidak memicu tambahan GET.
- [x] Ubah respons ConfigMap mock antara dua pemanggilan; assert nilai pertama/kedua berbeda dan ada dua GET untuk nama yang sama.
- [x] Jalankan `cargo test` dan `cargo build`; nol warning.

### Task 3 — UI lengkap dan tes pengguna

File: `src/app/pod_details.rs`; update fixture yang relevan, tanpa perubahan UI lain.

- [x] Uji issue-only container tetap mempunyai disclosure dan pesan yang sesuai.
- [x] Uji nilai panjang/multiline bisa dibuka, dipilih/disalin penuh.
- [x] Uji empty string, unavailable, duplicate names (resolver), dan reference source.
- [x] Uji masking Secret tetap berlaku termasuk selectable text/clipboard.
- [x] Tambahkan keterangan bahwa nilai ConfigMap adalah konfigurasi saat ini.
- [x] UI test nama environment panjang dengan nilai ConfigMap pendek: nama dibatasi dan diberi tooltip, tombol/badge tetap dalam batas drawer, nilai dapat dibuka/disalin. Unit test helper yang dipakai observer memverifikasi disclosure/detail/conditions dibersihkan saat scope Pod berubah.
- [ ] Owner memeriksa drawer pada lebar minimum dan tema light/dark secara manual.
- [x] Jalankan seluruh `cargo test` dan `cargo build`; nol warning.

### Task 4 — Pemeriksaan owner dan dokumentasi

- [ ] Owner menjalankan app dan membandingkan dengan fixture ConfigMap yang diketahui: envFrom, prefix, keyRef, multi-container, empty/multiline/long value.
- [ ] Periksa drawer pada lebar minimum, light/dark, banyak key, dan copy data lengkap.
- [ ] Periksa akses ConfigMap ditolak/tidak ditemukan tanpa mengubah cluster produksi atau permission otomatis; pakai mock/test namespace yang disetujui owner.
- [x] Perbarui spec migrasi bagian Pod drawer untuk mencatat fitur tambahan dan batas current configuration vs running process.
- [ ] Setelah owner meminta commit, pisahkan fitur ini dari commit Tasks 10–13 yang masih belum di-commit; jangan men-stage seluruh worktree.

## Gate pengerjaan

Implementasi Tasks 1–3 dan pembaruan spec selesai. Owner masih perlu menjalankan aplikasi dan pemeriksaan visual/cluster manual; jangan commit/stage/stash/push/merge tanpa instruksi owner. Jangan menyentuh `/Applications/PodLogs.app` atau `~/.kube/config`. Pertahankan perubahan migrasi dan perbaikan Search Logs yang sudah ada.

## Referensi primer

- Kubernetes: Configure a Pod to Use a ConfigMap — `https://kubernetes.io/docs/tasks/configure-pod-container/configure-pod-configmap/`
- Kubernetes: Updating Configuration via a ConfigMap — `https://kubernetes.io/docs/tutorials/configuration/updating-configuration-via-a-configmap/`
- Kubernetes Pod API, env/envFrom precedence — `https://kubernetes.io/docs/reference/kubernetes-api/core/pod-v1/`

Referensi diperiksa 2026-11-12. Prioritas runtime Kubernetes: envFrom terakhir menang untuk nama duplikat, dan deklarasi env eksplisit mengalahkan envFrom. Plan ini mempertahankan sumber/deklarasi untuk inspeksi dan tidak mensimulasikan environment runtime atau membaca Secret untuk menyelesaikan prioritas tersebut.
