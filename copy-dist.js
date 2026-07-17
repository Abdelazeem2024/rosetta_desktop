// نسخ ملف الواجهة إلى dist مع BOM لضمان ظهور العربية صح في WebView2
const fs = require('fs');
const path = require('path');

const distDir = path.join(__dirname, 'dist');
if (!fs.existsSync(distDir)) fs.mkdirSync(distDir, { recursive: true });

// BOM = Byte Order Mark — يُجبر WebView2 على قراءة الملف كـ UTF-8
const BOM = '\uFEFF';
const content = fs.readFileSync(path.join(__dirname, 'index.html'), 'utf8');
fs.writeFileSync(path.join(distDir, 'index.html'), BOM + content, 'utf8');

console.log('✅ تم نسخ index.html إلى dist/ مع UTF-8 BOM');
