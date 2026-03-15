#!/usr/bin/env node
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const projectRoot = path.resolve(__dirname, "..");
const tauriDir = path.join(projectRoot, "src-tauri");
const targetDir = path.join(tauriDir, "target");
const markerPath = path.join(targetDir, ".build-path");

let storedPath = null;
try {
  storedPath = fs.readFileSync(markerPath, "utf8").trim();
} catch {}

if (storedPath !== projectRoot) {
  const hasArtifacts =
    fs.existsSync(path.join(targetDir, "debug")) ||
    fs.existsSync(path.join(targetDir, "release"));

  if (hasArtifacts) {
    console.log("Build path changed, cleaning Rust build cache...");
    execSync("cargo clean", { cwd: tauriDir, stdio: "inherit" });
  }

  fs.mkdirSync(targetDir, { recursive: true });
  fs.writeFileSync(markerPath, projectRoot);
}
