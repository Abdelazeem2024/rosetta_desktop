// نسخ ملف الواجهة الوحيد إلى مجلد dist الذي يقرأه Tauri
// مكتوب بـ Node.js البسيط ليعمل بدون فرق بين Windows/Mac/Linux
const fs = require('fs');
const path = require('path');

const distDir = path.join(__dirname, 'dist');
if (!fs.existsSync(distDir)) fs.mkdirSync(distDir, { recursive: true });

fs.copyFileSync(
  path.join(__dirname, 'index.html'),
  path.join(distDir, 'index.html')
);

console.log('✅ تم نسخ index.html إلى dist/ بنجاح');
