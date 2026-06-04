#!/usr/bin/env node
"use strict";

const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

const binary = process.platform === "win32" ? "vergabe-mcp.exe" : "vergabe-mcp";
const binaryPath = path.join(__dirname, "bin", binary);

if (!fs.existsSync(binaryPath)) {
  console.error(
    `vergabe-mcp binary not found at ${binaryPath}\n` +
      `Run "npm rebuild @qune-tech/vergabe-mcp" to trigger the download.`
  );
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: "inherit",
});

child.on("error", (err) => {
  console.error(`Failed to start vergabe-mcp: ${err.message}`);
  process.exit(1);
});

child.on("exit", (code) => {
  process.exit(code ?? 1);
});
