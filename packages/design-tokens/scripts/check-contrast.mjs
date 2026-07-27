#!/usr/bin/env node

const pairs = [
  ["light primary text", "#1a1d23", "#ffffff"],
  ["light secondary text", "#5a6072", "#ffffff"],
  ["dark primary text", "#e3e6ec", "#0e1014"],
  ["dark secondary text", "#b4bac8", "#0e1014"],
];

function luminance(hex) {
  const channels = hex.slice(1).match(/../g).map((value) => parseInt(value, 16) / 255);
  const linear = channels.map((value) =>
    value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4,
  );
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
}

function contrast(foreground, background) {
  const [lighter, darker] = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
  return (lighter + 0.05) / (darker + 0.05);
}

let failed = false;
for (const [label, foreground, background] of pairs) {
  const ratio = contrast(foreground, background);
  const passes = ratio >= 4.5;
  console.log(`${passes ? "PASS" : "FAIL"} ${label}: ${ratio.toFixed(2)}:1`);
  failed ||= !passes;
}

if (failed) process.exitCode = 1;
