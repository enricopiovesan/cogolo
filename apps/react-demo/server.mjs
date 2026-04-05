#!/usr/bin/env node

import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = fileURLToPath(new URL(".", import.meta.url));
const defaultPort = 4173;
const defaultAdapterBaseUrl = "http://127.0.0.1:4174";

const { port, adapterBaseUrl } = parseArgs(process.argv.slice(2));
const adapterOrigin = new URL(adapterBaseUrl);

const server = createServer(async (request, response) => {
  try {
    if (request.url?.startsWith("/local/browser-subscriptions")) {
      await proxyBrowserAdapter(request, response, adapterOrigin);
      return;
    }

    await serveStaticAsset(request, response);
  } catch (error) {
    response.statusCode = 500;
    response.setHeader("Content-Type", "text/plain; charset=utf-8");
    response.end(String(error));
  }
});

server.listen(port, "127.0.0.1", () => {
  console.log(`Traverse React demo serving on http://127.0.0.1:${port}`);
  console.log(`Proxying browser adapter requests to ${adapterOrigin.origin}`);
});

function parseArgs(args) {
  let port = defaultPort;
  let adapterBaseUrl = defaultAdapterBaseUrl;

  for (let index = 0; index < args.length; index += 1) {
    const current = args[index];
    if (current === "--port" && args[index + 1]) {
      port = Number(args[index + 1]);
      index += 1;
    } else if (current === "--adapter" && args[index + 1]) {
      adapterBaseUrl = args[index + 1];
      index += 1;
    }
  }

  if (!Number.isInteger(port) || port <= 0) {
    throw new Error(`invalid port: ${port}`);
  }

  return { port, adapterBaseUrl };
}

async function proxyBrowserAdapter(request, response, adapterOrigin) {
  const targetUrl = new URL(request.url, adapterOrigin);
  const headers = new Headers();

  for (const [name, value] of Object.entries(request.headers)) {
    if (value === undefined) {
      continue;
    }
    if (["host", "connection", "content-length"].includes(name.toLowerCase())) {
      continue;
    }
    headers.set(name, Array.isArray(value) ? value.join(", ") : value);
  }

  let body = undefined;
  if (!["GET", "HEAD"].includes(request.method || "")) {
    body = await readRequestBody(request);
  }

  const upstreamResponse = await fetch(targetUrl, {
    method: request.method,
    headers,
    body,
  });

  response.statusCode = upstreamResponse.status;
  response.statusMessage = upstreamResponse.statusText;

  upstreamResponse.headers.forEach((value, name) => {
    if (name.toLowerCase() === "content-length") {
      return;
    }
    response.setHeader(name, value);
  });

  if (!upstreamResponse.body) {
    response.end();
    return;
  }

  const reader = upstreamResponse.body.getReader();
  while (true) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    response.write(Buffer.from(value));
  }

  response.end();
}

async function serveStaticAsset(request, response) {
  const requestPath = request.url?.split("?")[0] ?? "/";
  const normalizedPath = normalize(requestPath).replace(/^([.][.][/\\])+/, "");
  const filePath = normalizedPath === "/" ? "index.html" : normalizedPath.replace(/^\//, "");
  const resolvedPath = join(rootDir, filePath);

  try {
    const contents = await readFile(resolvedPath);
    response.statusCode = 200;
    response.setHeader("Content-Type", contentTypeFor(resolvedPath));
    response.end(contents);
  } catch {
    if (requestPath !== "/") {
      const indexPath = join(rootDir, "index.html");
      const contents = await readFile(indexPath);
      response.statusCode = 200;
      response.setHeader("Content-Type", "text/html; charset=utf-8");
      response.end(contents);
      return;
    }

    response.statusCode = 404;
    response.setHeader("Content-Type", "text/plain; charset=utf-8");
    response.end("Not found");
  }
}

function contentTypeFor(filePath) {
  switch (extname(filePath)) {
    case ".html":
      return "text/html; charset=utf-8";
    case ".js":
      return "text/javascript; charset=utf-8";
    case ".css":
      return "text/css; charset=utf-8";
    case ".json":
      return "application/json; charset=utf-8";
    default:
      return "application/octet-stream";
  }
}

async function readRequestBody(request) {
  const chunks = [];
  for await (const chunk of request) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }
  return Buffer.concat(chunks);
}
