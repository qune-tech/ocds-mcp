#!/usr/bin/env node
"use strict";

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const os = require("os");

const VERSION = require("./package.json").version;
const REPO = "qune-tech/ocds-mcp";
const BASE_URL = `https://github.com/${REPO}/releases/download/v${VERSION}`;

const PLATFORMS = {
  "linux-x64": {
    asset: "ocds-mcp-linux-x86_64.tar.gz",
    extracted: "ocds-mcp-linux-x86_64",
    binary: "ocds-mcp",
  },
  "darwin-arm64": {
    asset: "ocds-mcp-macos-arm64.tar.gz",
    extracted: "ocds-mcp-macos-arm64",
    binary: "ocds-mcp",
  },
  "win32-x64": {
    asset: "ocds-mcp-windows-x86_64.zip",
    extracted: "ocds-mcp-windows-x86_64.exe",
    binary: "ocds-mcp.exe",
  },
};

function getPlatformKey() {
  return `${process.platform}-${process.arch}`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    const request = (url) => {
      https
        .get(url, (res) => {
          if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
            request(res.headers.location);
            return;
          }
          if (res.statusCode !== 200) {
            reject(new Error(`Download failed: HTTP ${res.statusCode} for ${url}`));
            return;
          }
          const chunks = [];
          res.on("data", (chunk) => chunks.push(chunk));
          res.on("end", () => resolve(Buffer.concat(chunks)));
          res.on("error", reject);
        })
        .on("error", reject);
    };
    request(url);
  });
}

function extractTarGz(buffer, destDir) {
  const archivePath = path.join(destDir, "archive.tar.gz");
  fs.writeFileSync(archivePath, buffer);
  execSync(`tar xzf "${archivePath}" -C "${destDir}"`, { stdio: "ignore" });
  fs.unlinkSync(archivePath);
}

function extractZip(buffer, destDir) {
  const archivePath = path.join(destDir, "archive.zip");
  fs.writeFileSync(archivePath, buffer);
  // Use PowerShell on Windows, unzip on Unix
  if (process.platform === "win32") {
    execSync(
      `powershell -Command "Expand-Archive -Force '${archivePath}' '${destDir}'"`,
      { stdio: "ignore" }
    );
  } else {
    execSync(`unzip -o "${archivePath}" -d "${destDir}"`, { stdio: "ignore" });
  }
  fs.unlinkSync(archivePath);
}

async function main() {
  const key = getPlatformKey();
  const info = PLATFORMS[key];

  if (!info) {
    console.error(
      `Unsupported platform: ${key}\n` +
        `Supported: ${Object.keys(PLATFORMS).join(", ")}\n` +
        `You can build from source: https://github.com/${REPO}#building-from-source`
    );
    process.exit(1);
  }

  const binDir = path.join(__dirname, "bin");
  const binaryPath = path.join(binDir, info.binary);

  // Skip download if binary already exists (cached)
  if (fs.existsSync(binaryPath)) {
    console.log(`ocds-mcp binary already exists at ${binaryPath}`);
    return;
  }

  const url = `${BASE_URL}/${info.asset}`;
  console.log(`Downloading ocds-mcp v${VERSION} for ${key}...`);
  console.log(`  ${url}`);

  let buffer;
  try {
    buffer = await download(url);
  } catch (err) {
    console.error(`Failed to download ocds-mcp binary: ${err.message}`);
    console.error(
      `\nYou can download it manually from:\n  https://github.com/${REPO}/releases/tag/v${VERSION}`
    );
    process.exit(1);
  }

  fs.mkdirSync(binDir, { recursive: true });

  if (info.asset.endsWith(".tar.gz")) {
    extractTarGz(buffer, binDir);
  } else if (info.asset.endsWith(".zip")) {
    extractZip(buffer, binDir);
  }

  // Rename extracted binary to canonical name
  const extractedPath = path.join(binDir, info.extracted);
  if (fs.existsSync(extractedPath) && info.extracted !== info.binary) {
    fs.renameSync(extractedPath, binaryPath);
  }

  // Make binary executable on Unix
  if (process.platform !== "win32") {
    fs.chmodSync(binaryPath, 0o755);
  }

  if (!fs.existsSync(binaryPath)) {
    console.error(
      `Binary not found after extraction. Expected: ${binaryPath}\n` +
        `Please report this issue at https://github.com/${REPO}/issues`
    );
    process.exit(1);
  }

  console.log(`ocds-mcp v${VERSION} installed to ${binaryPath}`);
}

main();
