# التثبيت والتشغيل

## المستخدم النهائي
حمّل `*-setup.exe` من تاب **Releases** في الريبو وشغّله. يحتاج Windows 10/11 مع WebView2.

## المطوّر (PowerShell)
المتطلبات: Rust (stable)، Node.js 20+، Microsoft C++ Build Tools، WebView2.

```powershell
npm install
npx tauri icon app-icon.svg
npm run tauri dev
```

## اختبارات
```powershell
npm run typecheck
cargo test --manifest-path src-tauri/Cargo.toml
```

## البناء
```powershell
npm run tauri build
```
الناتج في `src-tauri\target\release\` وفي `bundle\nsis\`.

## موقع البيانات
`%APPDATA%\com.shawady.pos\` (قاعدة البيانات `pos.db` ومجلد `logs`).
