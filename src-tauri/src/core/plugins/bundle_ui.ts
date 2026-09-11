import * as esbuild from "npm:esbuild@0.25.12";

const entry = Deno.args[0];
const outfile = Deno.args[1];
if (!entry || !outfile) {
  console.error("usage: bundle_ui.ts <entry> <outfile>");
  Deno.exit(1);
}

await esbuild.build({
  absWorkingDir: Deno.cwd(),
  bundle: true,
  entryPoints: [entry],
  format: "esm",
  legalComments: "none",
  loader: { ".css": "text" },
  logLevel: "warning",
  outfile,
  platform: "browser",
  target: ["es2022"],
});
await esbuild.stop();

// Headless smoke check: import the bundled ESM (without calling activate(), which
// may touch DOM) to catch top-level errors and typos/missing exports before the
// user ever enables the plugin. esbuild only proves the syntax is valid JS.
try {
  const mod = await import(`file://${Deno.realPathSync(outfile)}`);
  if (typeof mod.activate !== "function") {
    console.error("SMOKE_CHECK_FAILED: bundled module has no exported `activate` function");
    Deno.exit(1);
  }
} catch (err) {
  console.error(`SMOKE_CHECK_FAILED: importing the bundled module threw: ${err}`);
  Deno.exit(1);
}
