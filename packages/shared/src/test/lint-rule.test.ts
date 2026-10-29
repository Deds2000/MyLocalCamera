// @vitest-environment node
/// <reference types="node" />
import path from "node:path";
import { fileURLToPath } from "node:url";
import { ESLint } from "eslint";
import { expect, it } from "vitest";

const here = path.dirname(fileURLToPath(import.meta.url));
// packages/shared/src/test -> raiz do repositório (4 níveis acima)
const root = path.resolve(here, "../../../..");

// timeout explícito: o cold start do ESLint (carregar config + plugins) mediu ~7 s nesta máquina;
// o default de 5 s do Vitest fazia o teste flacar (F0-GATE, 2026-10-29)
it("UT-0.04: texto fixo em JSX é reprovado pelo lint", async () => {
  const eslint = new ESLint({
    cwd: root,
    overrideConfigFile: path.join(root, "eslint.config.js"),
  });
  // identificador minúsculo de propósito: a regra ignora literais em `const` MAIÚSCULAS
  const code = "export const a = () => <div>Texto fixo</div>;\n";
  const results = await eslint.lintText(code, {
    filePath: path.join(root, "desktop/src/__fixture__.tsx"),
  });
  expect(results[0]?.messages.some((m) => m.ruleId === "i18next/no-literal-string")).toBe(true);
}, 20_000);
