// نسخ ملف الواجهة إلى dist مع BOM لضمان ظهور العربية صح في WebView2
// + دمج ملف الترخيص license.cjs داخل الواجهة كما هو بدون أي تعديل عليه
const fs = require('fs');
const path = require('path');

const distDir = path.join(__dirname, 'dist');
if (!fs.existsSync(distDir)) fs.mkdirSync(distDir, { recursive: true });

// BOM = Byte Order Mark — يُجبر WebView2 على قراءة الملف كـ UTF-8
const BOM = '\uFEFF';
let content = fs.readFileSync(path.join(__dirname, 'index.html'), 'utf8');

// ── دمج ملف الترخيص (license.cjs) ─────────────────────────────
// لو الملف ناقص نوقف البناء فوراً برسالة واضحة، بدل ما نُخرج برنامجاً لا يمكن تفعيله
const licPath = path.join(__dirname, 'license.cjs');
if (!fs.existsSync(licPath)) {
  console.error('❌ ملف license.cjs غير موجود بجانب index.html — ضعه في جذر المشروع ثم أعد البناء.');
  process.exit(1);
}
const licSource = fs.readFileSync(licPath, 'utf8');
if (/<\/script/i.test(licSource) || licSource.includes('<!--')) {
  console.error('❌ ملف license.cjs يحتوي على نص غير مسموح داخل الصفحة (</script أو <!--).');
  process.exit(1);
}
const MARK = '/*__LICENSE_FILE__*/';
if (content.split(MARK).length !== 2) {
  console.error('❌ علامة دمج الترخيص غير موجودة (أو مكررة) في index.html.');
  process.exit(1);
}
content = content.replace(MARK, () => licSource); // دالة بدل نص: حتى لا تُفسَّر رموز $ داخل الملف

fs.writeFileSync(path.join(distDir, 'index.html'), BOM + content, 'utf8');

console.log('✅ تم نسخ index.html إلى dist/ مع UTF-8 BOM');
console.log('✅ تم دمج ملف الترخيص license.cjs داخل الواجهة (' + licSource.length + ' حرف)');
