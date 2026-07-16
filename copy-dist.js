// نسخ ملف الواجهة إلى dist مع BOM + UTF-8 صريح
const fs = require('fs');
const path = require('path');

const distDir = path.join(__dirname, 'dist');
if (!fs.existsSync(distDir)) fs.mkdirSync(distDir, { recursive: true });

// إجبار UTF-8 مع BOM
const BOM = '\uFEFF';
let content = fs.readFileSync(path.join(__dirname, 'index.html'), 'utf8');

// تنظيف أي مشاكل encoding محتملة
content = content.replace(/[\u200B-\u200D\uFEFF]/g, '');

fs.writeFileSync(
  path.join(distDir, 'index.html'), 
  BOM + content, 
  { encoding: 'utf8' }
);

console.log('✅ تم نسخ index.html إلى dist/ مع UTF-8 BOM قوي');
