#!/usr/bin/env node
// F1-T01 — valida a coerência interna de docs/protocol-vectors.json.
// Checa (por vetor de frames): magic 43 4c no início de cada frame esperado,
// campo `length` (u32 LE, offset 14) == tamanho do payload esperado, e
// soma (18 + payload) de cada frame esperado == total de bytes - pendingBytes.
// Checa ainda: `name` não duplicado e todo item ter `expect`.
// Falhou → imprime o nome do vetor e sai com código 1.
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const file = path.join(root, "docs", "protocol-vectors.json");

let data;
try {
  data = JSON.parse(readFileSync(file, "utf8"));
} catch (err) {
  console.error(`erro: não consegui ler/parsear o arquivo de vetores: ${err.message}`);
  process.exit(1);
}

const failures = [];
const fail = (name, msg) => failures.push(`${name}: ${msg}`);

// hex (com ou sem espaços) → Uint8Array; null se inválido (falha já registrada)
function toBytes(hex, name) {
  const clean = hex.replace(/\s+/g, "");
  if (clean.length % 2 !== 0 || !/^[0-9a-fA-F]*$/.test(clean)) {
    fail(name, "hex inválido");
    return null;
  }
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Number.parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

const listNames = new Set();
for (const key of ["frames", "json"]) {
  const list = data[key];
  if (!Array.isArray(list)) {
    console.error(`erro: campo "${key}" ausente ou não é uma lista`);
    process.exit(1);
  }
  for (const item of list) {
    const name = item && item.name ? item.name : "<sem nome>";
    if (listNames.has(name)) fail(name, "name duplicado");
    listNames.add(name);
    if (item == null || item.expect === undefined || item.expect === null) {
      fail(name, "falta o campo expect");
    }
  }
}

const frames = data.frames;
const jsonList = data.json;

for (const item of frames) {
  const name = item && item.name ? item.name : "<sem nome>";
  if (typeof item.hex !== "string") {
    fail(name, "falta o campo hex");
    continue;
  }
  const bytes = toBytes(item.hex, name);
  if (!bytes) continue;
  const expect = item.expect;
  if (!expect || typeof expect !== "object") continue; // já reportado acima
  if (typeof expect.error === "string") continue; // vetor de erro: nada de conferir nos bytes
  if (!Array.isArray(expect.frames)) {
    fail(name, "expect sem lista frames");
    continue;
  }
  const pending = expect.pendingBytes ?? 0;
  const available = bytes.length - pending;
  let sum = 0;
  let off = 0;
  for (const frame of expect.frames) {
    const payloadLen = (frame.payloadHex ?? "").replace(/\s+/g, "").length / 2;
    if (bytes[off] !== 0x43 || bytes[off + 1] !== 0x4c) {
      fail(name, `magic != 43 4c no offset ${off}`);
    }
    const lenField =
      (bytes[off + 14] | (bytes[off + 15] << 8) | (bytes[off + 16] << 16) | (bytes[off + 17] << 24)) >>> 0;
    if (lenField !== payloadLen) {
      fail(name, `length do header (${lenField}) != payload esperado (${payloadLen}) no offset ${off}`);
    }
    sum += 18 + payloadLen;
    off += 18 + payloadLen;
  }
  if (sum !== available) {
    fail(name, `soma 18+payload = ${sum} != total de bytes ${bytes.length} - pendingBytes ${pending}`);
  }
}

if (failures.length > 0) {
  for (const f of failures) console.error(`erro no vetor ${f}`);
  process.exit(1);
}

console.log(`vetores OK: ${frames.length} de frames, ${jsonList.length} de json`);
