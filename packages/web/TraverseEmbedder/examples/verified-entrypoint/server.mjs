#!/usr/bin/env node
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const exampleRoot = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = fileURLToPath(new URL("../../../../../", import.meta.url));
const packageDist = join(repoRoot, "packages/web/TraverseEmbedder/dist");
const port = Number(process.env.PORT ?? "4176");

createServer(async (request, response) => {
  const path = request.url?.split("?")[0] ?? "/";
  const root = path.startsWith("/pkg/") ? packageDist : exampleRoot;
  const relative = path.startsWith("/pkg/") ? path.slice(5) : path === "/" ? "index.html" : path.slice(1);
  const filePath = join(root, normalize(relative).replace(/^([.][.][/\\])+/, ""));
  try {
    response.setHeader("Content-Type", contentType(filePath));
    response.end(await readFile(filePath));
  } catch {
    response.statusCode = 404;
    response.end("Not found");
  }
}).listen(port, "127.0.0.1", () => {
  console.log(`Verified-entrypoint browser demo: http://127.0.0.1:${port}`);
});

function contentType(filePath) {
  return extname(filePath) === ".html" ? "text/html; charset=utf-8"
    : extname(filePath) === ".js" ? "text/javascript; charset=utf-8"
    : extname(filePath) === ".css" ? "text/css; charset=utf-8"
    : "application/octet-stream";
}
