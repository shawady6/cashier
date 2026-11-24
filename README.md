# نظام نقاط البيع (POS)

برنامج كاشير احترافي يعمل **أوفلاين** على ويندوز للمحلات الصغيرة والمتوسطة (بقالة، سوبر ماركت، منظفات، عطور، تجزئة عامة).

**التقنيات:** Rust + Tauri 2 + React + TypeScript + Vite + Tailwind + SQLite (SQLx).

## الحالة
المرحلة 0 (الأساس): الكود مكتوب وبانتظار التحقق الفعلي بواسطة GitHub Actions. التفاصيل في `docs/ROADMAP.md`.

## تحميل ملف exe
كل push على `main` يشغّل GitHub Actions ويبني النسخة وينشرها في تاب **Releases** (ملف التثبيت `*-setup.exe` وملف `pos-system.exe`).

## التشغيل محليًا (PowerShell)
```powershell
npm install
npx tauri icon app-icon.svg
npm run tauri dev
```

## التوثيق
`docs/ARCHITECTURE.md` و`docs/DATABASE.md` و`docs/FEATURES.md` و`docs/ROADMAP.md` و`docs/INSTALLATION.md`، وقواعد العمل لـ Claude Code في `CLAUDE.md`.
