// Draws the README download buttons (docs/download-*.svg). Run: node assets/make_badges.js
// Apple and Linux glyphs come from Simple Icons (CC0); Windows is drawn as four squares.
const fs = require("fs");
const path = require("path");
const si = require("simple-icons");

const out = path.join(__dirname, "..", "docs");
const W = 236, H = 60, R = 12;
const font = "-apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif";

const glyphs = {
  macos: `<path transform="translate(20 18) scale(1)" fill="#fff" d="${si.siApple.path}"/>`,
  linux: `<path transform="translate(20 18) scale(1)" fill="#fff" d="${si.siLinux.path}"/>`,
  windows: `<g transform="translate(20 18)" fill="#fff"><rect x="0" y="0" width="11.2" height="11.2"/><rect x="12.8" y="0" width="11.2" height="11.2"/><rect x="0" y="12.8" width="11.2" height="11.2"/><rect x="12.8" y="12.8" width="11.2" height="11.2"/></g>`,
};

function badge(os, title, sub) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" role="img" aria-label="Download for ${title} (${sub})">
  <rect x=".75" y=".75" width="${W - 1.5}" height="${H - 1.5}" rx="${R}" fill="#111" stroke="#4a4f57" stroke-width="1.5"/>
  ${glyphs[os]}
  <text x="58" y="27" fill="#fff" font-family="${font}" font-size="15" font-weight="600">Download for ${title}</text>
  <text x="58" y="45" fill="#9aa0a6" font-family="${font}" font-size="12">${sub}</text>
</svg>
`;
}

const list = [
  ["macos-arm", "macos", "macOS", "Apple Silicon · .dmg"],
  ["macos-intel", "macos", "macOS", "Intel · .dmg"],
  ["windows", "windows", "Windows", "10 / 11 · installer .exe"],
  ["linux", "linux", "Linux", "x86_64 · AppImage"],
  ["linux-arm", "linux", "Linux", "ARM64 · AppImage"],
];
for (const [file, os, title, sub] of list) {
  fs.writeFileSync(path.join(out, `download-${file}.svg`), badge(os, title, sub));
  console.log(`docs/download-${file}.svg`);
}
