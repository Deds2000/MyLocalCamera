/** UT-3.13: vetores de `docs/pairing-uri-vectors.json`.
 * Roundtrip: `parse(build(value))` idêntico aos casos `ok`. */
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { buildPairingUri, parsePairingUri, type PairingUri } from "./pairing-uri";

type PairingVector = {
  name: string;
  uri: string;
  expect: "ok" | "invalid" | "incompatible_version";
  value: PairingUri | null;
};

const vectors: PairingVector[] = JSON.parse(
  readFileSync(
    resolve(dirname(fileURLToPath(import.meta.url)), "../../../../docs/pairing-uri-vectors.json"),
    "utf8",
  ),
) as PairingVector[];

describe("pairing uri", () => {
  it("pairing_uri_vectors", () => {
    for (const v of vectors) {
      const result = parsePairingUri(v.uri);
      if (v.expect === "ok") {
        expect(result.ok, `deveria passar: ${v.name}`).toBe(true);
        if (!result.ok) throw new Error(`deveria passar: ${v.name}`);
        expect(result.value, v.name).toEqual(v.value);
      } else {
        expect(result.ok, `deveria falhar: ${v.name}`).toBe(false);
        if (result.ok) throw new Error(`deveria falhar: ${v.name}`);
        expect(result.error, v.name).toBe(v.expect);
      }
    }
  });

  it("pairing_uri_roundtrip", () => {
    for (const v of vectors) {
      if (v.expect !== "ok" || v.value === null) continue;
      const result = parsePairingUri(buildPairingUri(v.value));
      expect(result.ok, `roundtrip deveria passar: ${v.name}`).toBe(true);
      if (!result.ok) throw new Error(`roundtrip deveria passar: ${v.name}`);
      expect(result.value, v.name).toEqual(v.value);
    }
  });
});
