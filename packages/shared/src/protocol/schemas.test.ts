/** UT-1.02: vetores `json` do `docs/protocol-vectors.json`.
 * UT-1.09: `welcome_unknown_reason` → `reason === "other"`. */
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  audioHelloSchema,
  byeSchema,
  helloSchema,
  setPayloadSchema,
  stateSchema,
  statsSchema,
  streamConfigSchema,
  welcomeSchema,
} from "./schemas";

type JsonVector = { name: string; message: string; json: unknown; expect: string };
type ProtocolVectors = { json: JsonVector[] };

const file: ProtocolVectors = JSON.parse(
  readFileSync(
    resolve(dirname(fileURLToPath(import.meta.url)), "../../../../docs/protocol-vectors.json"),
    "utf8",
  ),
) as ProtocolVectors;
const vectors: JsonVector[] = file.json;

const schemaByMessage: Record<string, { safeParse(v: unknown): { success: boolean } }> = {
  HELLO: helloSchema,
  WELCOME: welcomeSchema,
  STATE: stateSchema,
  SET: setPayloadSchema,
  STREAM_CONFIG: streamConfigSchema,
  AUDIO_HELLO: audioHelloSchema,
  STATS: statsSchema,
  BYE: byeSchema,
};

describe("protocol json vectors", () => {
  it("json_vectors_pass", () => {
    for (const item of vectors) {
      const schema = schemaByMessage[item.message];
      expect(schema, `schema para ${item.message} (vetor ${item.name})`).toBeDefined();
      if (schema === undefined) throw new Error(`schema ausente: ${item.message}`);
      const result = schema.safeParse(item.json);
      if (item.expect === "ok") {
        expect(result.success, `deveria passar: ${item.name}`).toBe(true);
      } else {
        expect(result.success, `deveria falhar: ${item.name}`).toBe(false);
      }
    }
  });

  it("unknown_reason_is_other_ts", () => {
    const item = vectors.find((v) => v.name === "welcome_unknown_reason");
    expect(item, "vetor welcome_unknown_reason ausente").toBeDefined();
    const parsed = welcomeSchema.safeParse(item!.json);
    expect(parsed.success).toBe(true);
    if (!parsed.success) throw new Error("deveria passar: welcome_unknown_reason");
    expect(parsed.data.reason).toBe("other");
  });
});
