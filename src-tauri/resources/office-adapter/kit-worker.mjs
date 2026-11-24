// Runs under the user-installed Node.js. The upstream package owns engine selection,
// font discovery, cancellation and native process teardown.
import { createRequire } from "node:module";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const resolveKit = createRequire(join(process.argv[2], "package.json"));
const { createConverter, ENGINE_VERSION } = await import(
  pathToFileURL(resolveKit.resolve("@deepseek-ai/libreoffice-kit")).href
);
let payload = "";
for await (const chunk of process.stdin) payload += chunk;
const input = JSON.parse(payload);
const controller = new AbortController();
process.on("SIGTERM", () => controller.abort());
process.on("SIGINT", () => controller.abort());
let converter;
try {
  converter = await createConverter({
    timeoutMs: input.timeoutMs,
    maxInputBytes: 128 * 1024 * 1024,
  });
  const request = { inputPath: input.input, outputPath: input.output };
  let result;
  if (input.operation === "recalculate") {
    result = await converter.recalculate(request, controller.signal);
  } else if (input.operation === "convert") {
    result = await converter.convert(request, controller.signal);
  } else if (input.operation === "render") {
    const pdf = await converter.render(request, controller.signal);
    const images = await converter.renderImages(
      {
        inputPath: input.input,
        outputDir: input.images,
        dpi: 110,
      },
      controller.signal,
    );
    result = {
      ...pdf,
      pages: images.images.map((image) => image.path),
      manifest: images,
      missingFonts: [...new Set([...(pdf.missingFonts ?? []), ...(images.missingFonts ?? [])])],
    };
  } else {
    throw new Error(`Unsupported Office operation: ${input.operation}`);
  }
  process.stdout.write(JSON.stringify({ ...result, engine: `LibreOffice Kit ${ENGINE_VERSION}` }));
} catch (error) {
  process.stderr.write(`${error.code ?? "OFFICE_ERROR"}: ${error.message}\n`);
  process.exitCode = 1;
} finally {
  await converter?.dispose();
}
