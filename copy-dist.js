const fs = require('fs');
const path = require('path');

const distDir = path.join(__dirname, 'dist');
if (!fs.existsSync(distDir)) fs.mkdirSync(distDir, { recursive: true });

// نسخ مباشر وبسيط — يحافظ على الترميز الأصلي
fs.copyFileSync(
  path.join(__dirname, 'index.html'),
  path.join(distDir, 'index.html')
);

console.log('✅ تم نسخ index.html إلى dist/ بنجاح');
