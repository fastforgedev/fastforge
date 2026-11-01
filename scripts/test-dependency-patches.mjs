import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const webRequire = createRequire(
  new URL("../apps/studio-web/package.json", import.meta.url),
);
const configRequire = createRequire(
  webRequire.resolve("@tanstack/eslint-config"),
);
const pluginPath = configRequire.resolve("eslint-plugin-import-x");
const plugins = {
  commonjs: configRequire("eslint-plugin-import-x"),
  esm: (await import(pathToFileURL(join(dirname(pluginPath), "index.js")).href))
    .default,
};
const { Linter } = webRequire("eslint");
const filename = fileURLToPath(
  new URL("../apps/studio-web/src/dependency-patches.test.js", import.meta.url),
);

for (const [format, plugin] of Object.entries(plugins)) {
  for (const [options, violations] of [
    [{ allow: ["{react,react-dom}/**"] }, 0],
    [{ allow: ["react-dom/**"] }, 1],
    [{ forbid: ["react/**"] }, 1],
    [{ forbid: ["react-dom/**"] }, 0],
  ]) {
    test(`${format} no-internal-modules respects ${JSON.stringify(options)}`, () => {
      const messages = new Linter().verify(
        "import 'react/jsx-runtime'\n",
        [
          {
            plugins: { "import-x": plugin },
            rules: { "import-x/no-internal-modules": ["error", options] },
          },
        ],
        { filename },
      );
      assert.equal(messages.length, violations, JSON.stringify(messages));
      for (const message of messages) {
        assert.equal(message.ruleId, "import-x/no-internal-modules");
        assert.equal(message.messageId, "noAllowed");
      }
    });
  }
}

const clientRequire = createRequire(
  new URL("../packages/studio-api-client/package.json", import.meta.url),
);
const generatorRequire = createRequire(
  clientRequire.resolve("openapi-typescript"),
);
const redoclyRequire = createRequire(
  generatorRequire.resolve("@redocly/openapi-core"),
);
const { readFileFromUrl } = redoclyRequire("./utils.js");

test("OpenAPI headers match the intended protocol and URL glob", async () => {
  const url = "https://api.example.com/v1/pets";
  const result = await readFileFromUrl(url, {
    headers: [
      { matches: "api.example.com/**", name: "X-Host", value: "host" },
      {
        matches: "https://api.example.com/v1/*",
        name: "X-Secure",
        value: "secure",
      },
      { matches: "http://api.example.com/**", name: "X-Http", value: "http" },
      { matches: "api.example.com/v2/*", name: "X-Other", value: "other" },
    ],
    customFetch: async (requestUrl, options) => {
      assert.equal(requestUrl, url);
      assert.deepEqual(options.headers, {
        "X-Host": "host",
        "X-Secure": "secure",
      });
      return new Response("schema", {
        headers: { "content-type": "application/yaml" },
      });
    },
  });
  assert.deepEqual(result, { body: "schema", mimeType: "application/yaml" });
});
