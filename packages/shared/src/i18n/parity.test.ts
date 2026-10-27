import { describe, expect, it } from "vitest";
import en from "./en.json";
import ptBR from "./pt-BR.json";

type JsonObject = Record<string, unknown>;

function flatten(obj: JsonObject, prefix = ""): string[] {
  const keys: string[] = [];
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === "object") {
      keys.push(...flatten(value as JsonObject, path));
    } else {
      keys.push(path);
    }
  }
  return keys.sort();
}

describe("UT-0.03 paridade i18n", () => {
  it("pt-BR e en têm o mesmo conjunto de chaves-folha", () => {
    expect(flatten(ptBR as JsonObject)).toEqual(flatten(en as JsonObject));
  });

  it("detecta diferença quando uma chave falta", () => {
    const enSemChave: JsonObject = JSON.parse(JSON.stringify(en));
    const smoke = (enSemChave.mobile as JsonObject).smoke as JsonObject;
    delete smoke.close;
    expect(flatten(enSemChave)).not.toEqual(flatten(ptBR as JsonObject));
  });
});
