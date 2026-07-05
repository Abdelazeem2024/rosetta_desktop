# Rosetta ERP — نسخة سطح المكتب (Tauri)

## محتوى المشروع
- `index.html` — نفس واجهة وكود البرنامج بالكامل، فقط طبقة التخزين تتكلم مع Rust/SQLite بدل المتصفح
- `src-tauri/` — الجزء الخاص بـ Rust (قاعدة بيانات SQLite حقيقية على القرص)
- `.github/workflows/build.yml` — يبني تطبيق Windows تلقائياً على سيرفرات GitHub المجانية

## أين تُحفظ البيانات؟
ملف `rosetta.db` (SQLite حقيقي) داخل مجلد بيانات التطبيق الخاص بالمستخدم على ويندوز:
```
C:\Users\<اسم المستخدم>\AppData\Roaming\com.rosetta.erp\rosetta.db
```
هذا الملف هو نسخة بياناتك الكاملة — يمكن نسخه احتياطياً مباشرة.

## البناء عبر GitHub (الطريقة المُوصى بها — بدون أي تثبيت برامج)
1. ارفع هذا المجلد كمستودع (Repository) جديد على github.com
2. GitHub سيبني نسخة Windows تلقائياً (راجع تبويب Actions)
3. حمّل ملف `.msi` أو `.exe` الجاهز من تبويب Releases

## البناء محلياً (لمن يفضّل ذلك)
```
npm install
npm run tauri build
```
يتطلب: Node.js 18+ و Rust (rustup.rs)
